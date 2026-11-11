//! Keeps the feet of a model on the floor.
//!
//! A hook on the skinning product of the game checks the bone of a foot and, when the foot sinks into the floor, turns it about the ankle until it rests on the floor.
//! Only bones whose inverse bind data carries the `FEET` tag are touched.

use recomp_abi::Context;
use zakuro_cpu::Bus;

use crate::memory::Memory;
use crate::recompiled::AfterHook;

/// The address of the function of the game that multiplies a world matrix by an inverse bind matrix to give a skinning matrix.
pub const SKINNING_PRODUCT: u32 = 0x0030_6964;
/// The return address inside the skinning loop of the game, which tells that the call comes from that loop.
const SKINNING_LOOP_RETURN: u32 = 0x0026_07CC;

/// The marker that a foot bone carries in its inverse bind data.
const FOOT_TAG: u32 = u32::from_le_bytes(*b"FEET");
/// The offset of the marker from the start of the inverse bind data, which is right after the three foot points of 12 bytes each.
const FOOT_TAG_OFFSET: u32 = 36;
/// The distance from the start of the bone record to its inverse bind data.
const INVERSE_BIND_OFFSET: u32 = 0xA4;
/// The offset of the index of the bone inside the bone record.
const BONE_INDEX_OFFSET: u32 = 0x8;
/// The offset of the index of the parent bone inside the bone record.
const PARENT_INDEX_OFFSET: u32 = 0xC;
/// The size of a 3x4 matrix of floats.
const MATRIX_BYTES: u32 = 48;
/// The number of halvings used to search the angle at which the toe touches the floor.
const BISECTION_STEPS: usize = 24;

/// A 3x4 affine matrix with three rows and the translation in the last column.
type Matrix = [[f32; 4]; 3];
/// A point or a direction in three dimensions.
type Vector = [f32; 3];

/// The hook on the skinning product.
/// It replaces the skinning matrix of a tagged foot bone with one that is turned about the ankle so that the foot does not sink into the floor.
/// Any other call runs the original function.
pub fn keep_feet_on_the_ground(memory: &mut Memory, ctx: &mut Context) -> AfterHook {
    // The arguments are the output skinning matrix, the world matrix and the inverse bind data.
    let (skinning_matrix, world_matrix, inverse_bind) = (ctx.r[0], ctx.r[1], ctx.r[2]);
    // Only the call from the skinning loop is handled, and only for a bone that carries the tag.
    if ctx.r[14] != SKINNING_LOOP_RETURN || memory.read32(inverse_bind.wrapping_add(FOOT_TAG_OFFSET)) != FOOT_TAG {
        return AfterHook::RunOriginal;
    }
    let bone_record = inverse_bind.wrapping_sub(INVERSE_BIND_OFFSET);
    let bone = memory.read32(bone_record + BONE_INDEX_OFFSET);
    let parent = memory.read32(bone_record + PARENT_INDEX_OFFSET);
    if parent >= bone {
        return AfterHook::RunOriginal;
    }
    // The matrices of the bones are stored one after the other, so the skinning matrix of the parent and the world matrix of the first bone are found by stepping back from the current ones.
    let shin = read_matrix(memory, skinning_matrix.wrapping_sub((bone - parent) * MATRIX_BYTES));
    let model = read_matrix(memory, world_matrix.wrapping_sub(bone * MATRIX_BYTES));
    let [ankle, toe, heel] = read_foot_points(memory, inverse_bind).map(|point| apply(&shin, point));
    // The second column of the model matrix is taken as the up direction, and the fourth as the floor.
    let up = normalize(column(&model, 1));
    let floor = column(&model, 3);
    let foot = match turn_onto_floor(ankle, toe, heel, up, floor) {
        Some((axis, angle)) => turned_about(&shin, axis, angle, ankle),
        None => shin,
    };
    write_matrix(memory, skinning_matrix, &foot);
    AfterHook::ReturnToCaller
}

/// Reads the ankle, toe and heel points of the foot, nine floats at the start of the inverse bind data.
fn read_foot_points(memory: &mut Memory, inverse_bind: u32) -> [Vector; 3] {
    let mut points = [[0.0; 3]; 3];
    for (point_index, point) in points.iter_mut().enumerate() {
        for (axis, value) in point.iter_mut().enumerate() {
            *value = f32::from_bits(memory.read32(inverse_bind + 4 * (3 * point_index + axis) as u32));
        }
    }
    points
}

/// Finds the axis and the angle that turn the foot about the ankle so that it no longer sinks into the floor.
/// Returns `None` when the toe and the heel are already above the floor, or when the sole is parallel to the up direction.
fn turn_onto_floor(ankle: Vector, toe: Vector, heel: Vector, up: Vector, floor: Vector) -> Option<(Vector, f32)> {
    // The height of a point above the floor along the up direction.
    let height = |point: Vector| dot(up, sub(point, floor));
    if height(toe) >= 0.0 && height(heel) >= 0.0 {
        return None;
    }
    let sole = sub(toe, heel);
    let across = cross(up, sole);
    // A sole that points straight up or down has no usable turning axis.
    if dot(across, across) < 1e-12 {
        return None;
    }
    let axis = normalize(across);
    let rise = dot(up, sole);
    // The angle at which the sole is perpendicular to the up direction, which lays the foot flat.
    let angle_to_flat = (rise / dot(sole, sole).sqrt()).clamp(-1.0, 1.0).asin();
    if rise >= 0.0 {
        return Some((axis, angle_to_flat));
    }
    let toe_height_at = |angle: f32| height(add(ankle, rotate(sub(toe, ankle), axis, angle)));
    if toe_height_at(angle_to_flat) <= 0.0 {
        return Some((axis, angle_to_flat));
    }
    // The toe is the low point and laying the foot flat would leave it above the floor, so the angle is searched at which the toe touches the floor.
    let (mut toe_under_floor, mut toe_over_floor) = (0.0, angle_to_flat);
    for _ in 0..BISECTION_STEPS {
        let middle = (toe_under_floor + toe_over_floor) / 2.0;
        if toe_height_at(middle) < 0.0 {
            toe_under_floor = middle;
        } else {
            toe_over_floor = middle;
        }
    }
    Some((axis, toe_over_floor))
}

/// Turns a matrix about an axis through the pivot point.
fn turned_about(matrix: &Matrix, axis: Vector, angle: f32, pivot: Vector) -> Matrix {
    let mut turned = [[0.0; 4]; 3];
    // The three columns of the rotation part are turned, and the translation is turned about the pivot.
    for column_index in 0..3 {
        let turned_column = rotate(column(matrix, column_index), axis, angle);
        for row in 0..3 {
            turned[row][column_index] = turned_column[row];
        }
    }
    let origin = add(pivot, rotate(sub(column(matrix, 3), pivot), axis, angle));
    for row in 0..3 {
        turned[row][3] = origin[row];
    }
    turned
}

/// Rotates a vector about a unit axis by an angle in radians, with the formula of Rodrigues.
fn rotate(vector: Vector, axis: Vector, angle: f32) -> Vector {
    let (sin, cos) = angle.sin_cos();
    let across = cross(axis, vector);
    let along = dot(axis, vector) * (1.0 - cos);
    [0, 1, 2].map(|i| vector[i] * cos + across[i] * sin + axis[i] * along)
}

/// One column of a matrix.
fn column(matrix: &Matrix, index: usize) -> Vector {
    [matrix[0][index], matrix[1][index], matrix[2][index]]
}

/// Reads a 3x4 matrix of twelve floats from the memory of the game, row by row.
fn read_matrix(memory: &mut Memory, address: u32) -> Matrix {
    let mut matrix = [[0.0; 4]; 3];
    for (i, value) in matrix.as_flattened_mut().iter_mut().enumerate() {
        *value = f32::from_bits(memory.read32(address + 4 * i as u32));
    }
    matrix
}

/// Writes a 3x4 matrix of twelve floats to the memory of the game, row by row.
fn write_matrix(memory: &mut Memory, address: u32, matrix: &Matrix) {
    for (i, value) in matrix.as_flattened().iter().enumerate() {
        memory.write32(address + 4 * i as u32, value.to_bits());
    }
}

/// Transforms a point by a matrix.
fn apply(matrix: &Matrix, point: Vector) -> Vector {
    [0, 1, 2].map(|i| matrix[i][0] * point[0] + matrix[i][1] * point[1] + matrix[i][2] * point[2] + matrix[i][3])
}

/// The sum of two vectors.
fn add(a: Vector, b: Vector) -> Vector {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// The difference of two vectors.
fn sub(a: Vector, b: Vector) -> Vector {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// The dot product of two vectors.
fn dot(a: Vector, b: Vector) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The cross product of two vectors.
fn cross(a: Vector, b: Vector) -> Vector {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// The vector scaled to length one, or the vector itself when its length is zero.
fn normalize(vector: Vector) -> Vector {
    let length = dot(vector, vector).sqrt();
    if length == 0.0 {
        return vector;
    }
    [vector[0] / length, vector[1] / length, vector[2] / length]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The up direction of the tests.
    const UP: Vector = [0.0, 1.0, 0.0];
    /// A point on the floor of the tests.
    const FLOOR: Vector = [0.0, 0.0, 0.0];

    /// Places a foot with the ankle at the height, leans it by the angle in degrees, and returns the toe and the heel after the correction.
    fn stand(lean: f32, height: f32) -> (Vector, Vector) {
        let (sin, cos) = lean.to_radians().sin_cos();
        let ankle = [0.0, height, 0.0];
        let lean = |p: Vector| add(ankle, [p[0], p[1] * cos - p[2] * sin, p[1] * sin + p[2] * cos]);
        let (toe, heel) = (lean([0.0, -0.8, 1.5]), lean([0.0, -0.8, -0.4]));
        match turn_onto_floor(ankle, toe, heel, UP, FLOOR) {
            Some((axis, angle)) => (add(ankle, rotate(sub(toe, ankle), axis, angle)), add(ankle, rotate(sub(heel, ankle), axis, angle))),
            None => (toe, heel),
        }
    }

    /// A foot above the floor is left as it is.
    #[test]
    fn a_foot_off_the_floor_keeps_its_angle_to_the_shin() {
        let (toe, heel) = stand(15.0, 2.0);
        assert!(toe[1] > 0.0 && heel[1] > 0.0);
        let sin = 15f32.to_radians().sin();
        assert!((toe[1] - (2.0 - 0.8 * 15f32.to_radians().cos() - 1.5 * sin)).abs() < 1e-5, "{toe:?}");
    }

    /// A toe that sinks into the floor is lifted until it touches it.
    #[test]
    fn a_toe_that_would_dig_in_comes_up_to_the_floor() {
        let (toe, heel) = stand(15.0, 0.9);
        assert!(toe[1].abs() < 1e-4, "the toe touches, {toe:?}");
        assert!(heel[1] > toe[1], "the heel off the floor, {heel:?}");
    }

    /// A heel that sinks into the floor lays the foot flat.
    #[test]
    fn a_heel_that_would_dig_in_lays_the_foot_flat() {
        let (toe, heel) = stand(-15.0, 0.85);
        assert!((toe[1] - heel[1]).abs() < 1e-4, "flat, {toe:?} {heel:?}");
        assert!(heel[1] > -0.1 && heel[1] < 0.1, "on the floor, {heel:?}");
    }

    /// A foot whose ankle is too low is still laid flat.
    #[test]
    fn a_foot_on_the_floor_lies_flat() {
        let (toe, heel) = stand(15.0, 0.7);
        assert!((toe[1] - heel[1]).abs() < 1e-4, "flat, {toe:?} {heel:?}");
        assert!(toe[1] < 0.0, "under the floor still, the ankle being too low");
    }

    /// The turned matrix keeps the ankle in place and swings the toe about it.
    #[test]
    fn the_turned_matrix_moves_the_foot_about_the_ankle() {
        let m = [[1.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 2.0], [0.0, 0.0, 1.0, 3.0]];
        let ankle = [1.0, 2.0, 3.0];
        let out = turned_about(&m, [1.0, 0.0, 0.0], 0.3, ankle);
        assert!(sub(apply(&out, [0.0, 0.0, 0.0]), ankle).iter().all(|d| d.abs() < 1e-6), "the ankle stays");
        let toe = apply(&out, [0.0, 0.0, 1.0]);
        let expected = add(ankle, rotate([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], 0.3));
        assert!(sub(toe, expected).iter().all(|d| d.abs() < 1e-6));
    }
}
