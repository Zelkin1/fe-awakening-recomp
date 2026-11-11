//! Converts a legacy PyTorch `.pth` ESRGAN model into ncnn files.
//! The weights are read with a small pickle reader, and the ncnn graph of the 4x model is edited into a 1x graph.

use std::collections::{BTreeSet, HashMap};

/// The magic number that opens a PyTorch file of the old kind, as the little-endian bytes of a long integer.
const MAGIC: [u8; 10] = [0x6c, 0xfc, 0x9c, 0x46, 0xf9, 0x20, 0x6a, 0xa8, 0x50, 0x19];
/// The number of residual blocks in the trunk of the network.
const BLOCKS: usize = 23;
/// The layers of the 4x graph that upscale, which the 1x graph drops.
const DROPPED: [&str; 4] = ["Resize_1177", "Conv_1178", "Resize_1181", "Conv_1182"];
/// The high resolution convolution that follows the dropped layers.
const HR_CONVOLUTION: &str = "Conv_1184";
/// The blob that holds the output of the trunk, which the high resolution convolution reads in the 1x graph.
const TRUNK_SUM: &str = "1878";
/// The ncnn line of the layer that enlarges the final output to twice the size.
const DOUBLING: &str = "Interp                   Resize_out               1 1 x1 output 0=1 1=2.000000e+00 2=2.000000e+00";

/// Converts a 1x model into an ncnn `.param` text and a `.bin` of weights.
/// The template is the `.param` text of the 4x graph, which is edited to fit the 1x weights.
pub fn one_x(template: &str, pth: &[u8]) -> Result<(String, Vec<u8>), String> {
    let state = read_legacy(pth)?;
    let convolutions = in_graph_order(&state)?;
    let param = one_x_param(template)?;
    let bin = weights(&param, &convolutions)?;
    Ok((param, bin))
}

/// A value on the stack of the pickle reader.
#[derive(Clone, Debug, PartialEq)]
enum Value {
    None,
    Bool(bool),
    Int(i64),
    /// An integer too large for 64 bits, as its little-endian bytes.
    Big(Vec<u8>),
    Text(String),
    Tuple(Vec<Value>),
    List(Vec<Value>),
    /// A dictionary, kept as a list of pairs in file order.
    Dict(Vec<(Value, Value)>),
    /// A reference to a named function or class, as `module.name`.
    Global(String),
    /// A call of a global with arguments, which is stored and not run.
    Call(String, Vec<Value>),
    /// A reference to a storage of floats by its key.
    Storage(String),
    /// The mark that opens a group of items on the stack.
    Mark,
}

/// A weight tensor with its values in row-major order.
struct Tensor {
    values: Vec<f32>,
    size: Vec<usize>,
}

/// A cursor over the bytes of the model file.
struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    /// Takes the next `count` bytes.
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let bytes = self.data.get(self.at..self.at + count).ok_or("the model file ends too soon")?;
        self.at += count;
        Ok(bytes)
    }

    /// Takes one byte.
    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    /// Takes a little-endian 16-bit number.
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    /// Takes a little-endian 32-bit number.
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    /// Takes a little-endian 64-bit number.
    fn i64(&mut self) -> Result<i64, String> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    /// Takes `count` bytes as a UTF-8 string value.
    fn text(&mut self, count: usize) -> Result<Value, String> {
        String::from_utf8(self.take(count)?.to_vec()).map(Value::Text).map_err(|_| "the model file holds a name that is not UTF-8".to_owned())
    }

    /// Takes a line of text up to a newline, and skips the newline.
    fn line(&mut self) -> Result<String, String> {
        let end = self.data[self.at..].iter().position(|&b| b == b'\n').ok_or("the model file ends too soon")?;
        let line = String::from_utf8_lossy(self.take(end)?).into_owned();
        self.at += 1;
        Ok(line)
    }
}

/// Reads one pickle from the reader, up to its stop operation.
/// It only understands the operations that PyTorch writes in the legacy format, and it does not run any code.
fn unpickle(reader: &mut Reader) -> Result<Value, String> {
    let mut stack: Vec<Value> = Vec::new();
    // The memo holds the values that the pickle stores to use again.
    let mut memo: HashMap<u32, Value> = HashMap::new();
    loop {
        let op = reader.byte()?;
        let value = match op {
            // The protocol operation, followed by the version, which is skipped.
            0x80 => {
                reader.byte()?;
                continue;
            }
            b'.' => return stack.pop().ok_or_else(|| "the model file has an empty pickle".to_owned()),
            b'(' => Value::Mark,
            b')' => Value::Tuple(Vec::new()),
            b']' => Value::List(Vec::new()),
            b'}' => Value::Dict(Vec::new()),
            b'N' => Value::None,
            0x88 => Value::Bool(true),
            0x89 => Value::Bool(false),
            b'K' => Value::Int(reader.byte()? as i64),
            b'M' => Value::Int(reader.u16()? as i64),
            b'J' => Value::Int(reader.u32()? as i32 as i64),
            // A long integer of up to 255 bytes, in two's complement.
            // It is sign-extended into 64 bits when it fits.
            0x8a => {
                let count = reader.byte()? as usize;
                let bytes = reader.take(count)?;
                if count <= 8 {
                    let fill = if bytes.last().is_some_and(|&b| b & 0x80 != 0) { 0xff } else { 0 };
                    let mut word = [fill; 8];
                    word[..count].copy_from_slice(bytes);
                    Value::Int(i64::from_le_bytes(word))
                } else {
                    Value::Big(bytes.to_vec())
                }
            }
            b'X' => {
                let count = reader.u32()? as usize;
                reader.text(count)?
            }
            0x8c => {
                let count = reader.byte()? as usize;
                reader.text(count)?
            }
            b't' => Value::Tuple(since_mark(&mut stack)?),
            // The tuples of one, two and three items.
            0x85..=0x87 => {
                let count = (op - 0x84) as usize;
                let start = stack.len().checked_sub(count).ok_or("the model file has a short tuple")?;
                Value::Tuple(stack.split_off(start))
            }
            // Add one item or a group of items to the list, or add key and value pairs to the dictionary, that is on the stack.
            b'a' | b'e' | b's' | b'u' => {
                let items = match op {
                    b'a' => vec![pop(&mut stack)?],
                    b's' => {
                        let value = pop(&mut stack)?;
                        vec![pop(&mut stack)?, value]
                    }
                    _ => since_mark(&mut stack)?,
                };
                match stack.last_mut() {
                    Some(Value::List(list)) => list.extend(items),
                    Some(Value::Dict(dict)) => {
                        let mut items = items.into_iter();
                        while let (Some(key), Some(value)) = (items.next(), items.next()) {
                            dict.push((key, value));
                        }
                    }
                    _ => return Err("the model file adds to something that is not a list or a dictionary".to_owned()),
                }
                continue;
            }
            // Store the top of the stack in the memo, and fetch a value from the memo.
            b'q' | b'r' => {
                let index = if op == b'q' { reader.byte()? as u32 } else { reader.u32()? };
                memo.insert(index, stack.last().cloned().ok_or("the model file keeps nothing")?);
                continue;
            }
            b'h' | b'j' => {
                let index = if op == b'h' { reader.byte()? as u32 } else { reader.u32()? };
                memo.get(&index).cloned().ok_or("the model file refers to something it never kept")?
            }
            b'c' => {
                let module = reader.line()?;
                let name = reader.line()?;
                Value::Global(format!("{module}.{name}"))
            }
            // A call: an ordered dictionary becomes an empty dictionary, and any other global is kept with its arguments.
            b'R' => {
                let arguments = pop(&mut stack)?;
                match (pop(&mut stack)?, arguments) {
                    (Value::Global(name), _) if name == "collections.OrderedDict" => Value::Dict(Vec::new()),
                    (Value::Global(name), Value::Tuple(arguments)) => Value::Call(name, arguments),
                    _ => return Err("the model file calls something unknown".to_owned()),
                }
            }
            // The build operation sets the state of an object, which is dropped.
            b'b' => {
                pop(&mut stack)?;
                continue;
            }
            // A persistent id, which PyTorch uses to point at a storage. Only storages of 32-bit floats are accepted.
            b'Q' => match pop(&mut stack)? {
                Value::Tuple(id) => match id.as_slice() {
                    [Value::Text(kind), Value::Global(storage), Value::Text(key), ..] if kind == "storage" && storage == "torch.FloatStorage" => Value::Storage(key.clone()),
                    _ => return Err("the model weights are not 32-bit floats".to_owned()),
                },
                _ => return Err("the model file has an unknown reference".to_owned()),
            },
            _ => return Err(format!("the model file uses pickle operation {op:#04x}, which Zakuro does not read")),
        };
        stack.push(value);
    }
}

/// Pops a value from the stack, or fails when it is empty.
fn pop(stack: &mut Vec<Value>) -> Result<Value, String> {
    stack.pop().ok_or_else(|| "the model file takes from an empty stack".to_owned())
}

/// Pops everything above the last mark, and the mark itself, and returns those items in order.
fn since_mark(stack: &mut Vec<Value>) -> Result<Vec<Value>, String> {
    let mark = stack.iter().rposition(|value| *value == Value::Mark).ok_or("the model file has no mark")?;
    let items = stack.split_off(mark + 1);
    stack.pop();
    Ok(items)
}

/// Reads a legacy PyTorch file into its tensors by name.
/// The file is a series of pickles: the magic number, the protocol version, the system information, the dictionary of weights and the list of storage keys.
/// The raw storages follow, each with its length as a 64-bit number and then its 32-bit floats.
fn read_legacy(pth: &[u8]) -> Result<HashMap<String, Tensor>, String> {
    let mut reader = Reader { data: pth, at: 0 };
    if unpickle(&mut reader)? != Value::Big(MAGIC.to_vec()) {
        return Err("the model is not a PyTorch file of the old kind".to_owned());
    }
    // The protocol version and the system information are not needed.
    unpickle(&mut reader)?;
    unpickle(&mut reader)?;
    let Value::Dict(state) = unpickle(&mut reader)? else { return Err("the model file holds no weights".to_owned()) };
    let Value::List(keys) = unpickle(&mut reader)? else { return Err("the model file lists no storages".to_owned()) };
    let mut storages = HashMap::new();
    for key in keys {
        let Value::Text(key) = key else { return Err("the model file names a storage oddly".to_owned()) };
        let count = usize::try_from(reader.i64()?).map_err(|_| "the model file has a storage of negative size")?;
        let values = reader.take(count * 4)?.chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect::<Vec<f32>>();
        storages.insert(key, values);
    }
    state
        .into_iter()
        .map(|(name, tensor)| {
            let Value::Text(name) = name else { return Err("the model file names a weight oddly".to_owned()) };
            let tensor = rebuild(&tensor, &storages).ok_or_else(|| format!("the model weight {name} can't be read"))?;
            Ok((name, tensor))
        })
        .collect()
}

/// Builds a tensor from a stored `_rebuild_tensor_v2` call, using its storage, offset, size and strides.
/// The strides are followed, so the values come out in row-major order whatever the layout was.
fn rebuild(tensor: &Value, storages: &HashMap<String, Vec<f32>>) -> Option<Tensor> {
    let Value::Call(function, arguments) = tensor else { return None };
    let [Value::Storage(key), Value::Int(offset), Value::Tuple(size), Value::Tuple(stride), ..] = arguments.as_slice() else { return None };
    if function != "torch._utils._rebuild_tensor_v2" {
        return None;
    }
    let numbers = |values: &[Value]| values.iter().map(|value| if let Value::Int(n) = value { usize::try_from(*n).ok() } else { None }).collect::<Option<Vec<usize>>>();
    let (size, stride, data) = (numbers(size)?, numbers(stride)?, storages.get(key)?);
    let count = size.iter().product::<usize>();
    let mut index = vec![0usize; size.len()];
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let at = usize::try_from(*offset).ok()? + index.iter().zip(&stride).map(|(i, s)| i * s).sum::<usize>();
        values.push(*data.get(at)?);
        // Step the index like an odometer, with the last dimension turning fastest.
        for d in (0..size.len()).rev() {
            index[d] += 1;
            if index[d] < size[d] {
                break;
            }
            index[d] = 0;
        }
    }
    Some(Tensor { values, size })
}

/// Returns the weight and bias of every convolution in the order that they appear in the ncnn graph.
/// That is the first convolution, the five of each of the three dense layers of each block, the end of the trunk, and then the two convolutions of the tail.
fn in_graph_order(state: &HashMap<String, Tensor>) -> Result<Vec<(&Tensor, &Tensor)>, String> {
    let mut order = vec!["model.0".to_owned()];
    for block in 0..BLOCKS {
        for dense in 1..=3 {
            for convolution in 1..=5 {
                order.push(format!("model.1.sub.{block}.RDB{dense}.conv{convolution}.0"));
            }
        }
    }
    order.push(format!("model.1.sub.{BLOCKS}"));
    // The convolutions after the trunk are numbered above 1. A 1x model has exactly two of them.
    let tail: BTreeSet<usize> = state.keys().filter_map(|key| key.strip_prefix("model.")?.split('.').next()?.parse().ok()).filter(|&n| n > 1).collect();
    if tail.len() != 2 {
        return Err("the model is not a 1x ESRGAN".to_owned());
    }
    order.extend(tail.iter().map(|n| format!("model.{n}")));
    order
        .iter()
        .map(|name| match (state.get(&format!("{name}.weight")), state.get(&format!("{name}.bias"))) {
            (Some(weight), Some(bias)) => Ok((weight, bias)),
            _ => Err(format!("the model has no convolution {name}")),
        })
        .collect()
}

/// Returns the whitespace-separated field of a line at an index, or an empty string.
fn field<'a>(line: &'a str, index: usize) -> &'a str {
    line.split_whitespace().nth(index).unwrap_or("")
}

/// Returns the line with one field replaced, and with the spacing normalised to single spaces.
fn with_field(line: &str, index: usize, value: &str) -> String {
    line.split_whitespace().enumerate().map(|(i, part)| if i == index { value } else { part }).collect::<Vec<_>>().join(" ")
}

/// Edits the `.param` text of the 4x graph into a 1x graph.
/// It drops the two upscaling stages, feeds the high resolution convolution from the trunk, renames the final blob and adds the doubling layer.
fn one_x_param(template: &str) -> Result<String, String> {
    let lines: Vec<&str> = template.lines().collect();
    // The first line is the magic number of the format, and the second line holds the counts of layers and blobs.
    let [magic, counts, ..] = lines.as_slice() else { return Err("the program's model description is empty".to_owned()) };
    // Four layers are dropped and one is added, so both counts shrink by three.
    let count = |index: usize| field(counts, index).parse::<usize>().ok().and_then(|n| n.checked_sub(3)).ok_or("the program's model description has no counts");
    let (layers, blobs) = (count(0)?, count(1)?);
    let mut body: Vec<String> = lines[2..].iter().filter(|line| !DROPPED.contains(&field(line, 1))).map(|line| line.to_string()).collect();
    let hr = body.iter().position(|line| field(line, 1) == HR_CONVOLUTION).ok_or("the program's model description has no HR convolution")?;
    // Field 4 is the input blob of the layer.
    body[hr] = with_field(&body[hr], 4, TRUNK_SUM);
    let last = body.last_mut().filter(|line| field(line, 5) == "output").ok_or("the program's model description ends oddly")?;
    // Field 5 is the output blob of the layer, which now feeds the doubling layer.
    *last = with_field(last, 5, "x1");
    body.push(DOUBLING.to_owned());
    Ok(format!("{magic}\n{layers} {blobs}\n{}\n", body.join("\n")))
}

/// Writes the ncnn `.bin` of weights for the convolutions of a graph.
/// Each convolution is stored as a zero tag for plain 32-bit floats, then the weights, then the biases.
fn weights(param: &str, convolutions: &[(&Tensor, &Tensor)]) -> Result<Vec<u8>, String> {
    let layers: Vec<&str> = param.lines().filter(|line| line.starts_with("Convolution")).collect();
    if layers.len() != convolutions.len() {
        return Err(format!("the model has {} convolutions where the program takes {}", convolutions.len(), layers.len()));
    }
    let mut bin = Vec::new();
    for (layer, (weight, bias)) in layers.iter().zip(convolutions) {
        // Parameter 0 is the number of outputs, which is the size of the bias, and parameter 6 is the size of the weights.
        let setting = |key: &str| layer.split_whitespace().find_map(|part| part.strip_prefix(key)?.parse::<usize>().ok());
        if setting("6=") != Some(weight.values.len()) || setting("0=") != Some(bias.values.len()) || weight.size.len() != 4 {
            return Err(format!("the model weights don't fit {}", field(layer, 1)));
        }
        bin.extend_from_slice(&0u32.to_le_bytes());
        for value in weight.values.iter().chain(&bias.values) {
            bin.extend_from_slice(&value.to_le_bytes());
        }
    }
    Ok(bin)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small pickle of a dictionary is read into the expected value.
    #[test]
    fn a_pickle_reads_back() {
        let pickle = b"\x80\x02}q\x00(X\x01\x00\x00\x00aq\x01K\x05X\x01\x00\x00\x00bq\x02]q\x03(\x88\x89eu.";
        let value = unpickle(&mut Reader { data: pickle, at: 0 }).unwrap();
        let expected = Value::Dict(vec![(Value::Text("a".into()), Value::Int(5)), (Value::Text("b".into()), Value::List(vec![Value::Bool(true), Value::Bool(false)]))]);
        assert_eq!(value, expected);
    }

    /// A small 4x graph is edited into the expected 1x graph.
    #[test]
    fn the_four_times_graph_becomes_one_times() {
        let template = "7767517\n9 10\nInput data 0 1 data\nConvolution Conv_1176 1 1 1877 1878 0=64 6=4\nInterp Resize_1177 1 1 1878 a\nConvolution Conv_1178 1 1 a b 0=64 6=4\nInterp Resize_1181 1 1 b c\nConvolution Conv_1182 1 1 c d 0=64 6=4\nConvolution Conv_1184 1 1 d e 0=64 6=4\nConvolution Conv_1186 1 1 e output 0=3 6=4\n";
        let expected = format!("7767517\n6 7\nInput data 0 1 data\nConvolution Conv_1176 1 1 1877 1878 0=64 6=4\nConvolution Conv_1184 1 1 1878 e 0=64 6=4\nConvolution Conv_1186 1 1 e x1 0=3 6=4\n{DOUBLING}\n");
        assert_eq!(one_x_param(template).unwrap(), expected);
    }

    /// The conversion of a real model gives the same files as the reference conversion.
    #[test]
    #[ignore = "ZAKURO_ESRGAN_PTH, ZAKURO_ESRGAN_TEMPLATE and ZAKURO_ESRGAN_EXPECTED, a folder with 1x-bcgone.param and .bin"]
    fn converts_as_the_reference_does() {
        let path = |name: &str| std::path::PathBuf::from(std::env::var(name).unwrap());
        let (param, bin) = one_x(&std::fs::read_to_string(path("ZAKURO_ESRGAN_TEMPLATE")).unwrap(), &std::fs::read(path("ZAKURO_ESRGAN_PTH")).unwrap()).unwrap();
        let expected = path("ZAKURO_ESRGAN_EXPECTED");
        assert_eq!(param, std::fs::read_to_string(expected.join("1x-bcgone.param")).unwrap());
        assert!(bin == std::fs::read(expected.join("1x-bcgone.bin")).unwrap());
    }
}
