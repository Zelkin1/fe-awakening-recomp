use recomp_abi::Context;
use zakuro_cpu::Bus;

use crate::memory::Memory;
use crate::recompiled::AfterHook;

pub const SKINNING_PRODUCT: u32 = 0x0030_6964;
const SKINNING_LOOP_RETURN: u32 = 0x0026_07CC;

const FOOT_TAG: u32 = u32::from_le_bytes(*b"FEET");
const FOOT_TAG_OFFSET: u32 = 36;
const INVERSE_BIND_OFFSET: u32 = 0xA4;
const BONE_INDEX_OFFSET: u32 = 0x8;
const PARENT_INDEX_OFFSET: u32 = 0xC;
const MATRIX_BYTES: u32 = 48;
const BISECTION_STEPS: usize = 24;

type Matrix = [[f32; 4]; 3];
type Vector = [f32; 3];

pub fn keep_feet_on_the_ground(memory: &mut Memory, ctx: &mut Context) -> AfterHook {
    let (skinning_matrix, world_matrix, inverse_bind) = (ctx.r[0], ctx.r[1], ctx.r[2]);
    if ctx.r[14] != SKINNING_LOOP_RETURN || memory.read32(inverse_bind.wrapping_add(FOOT_TAG_OFFSET)) != FOOT_TAG {
        return AfterHook::RunOriginal;
    }
    let bone_record = inverse_bind.wrapping_sub(INVERSE_BIND_OFFSET);
    let bone = memory.read32(bone_record + BONE_INDEX_OFFSET);
    let parent = memory.read32(bone_record + PARENT_INDEX_OFFSET);
    if parent >= bone {
        return AfterHook::RunOriginal;
    }
    let shin = read_matrix(memory, skinning_matrix.wrapping_sub((bone - parent) * MATRIX_BYTES));
    let model = read_matrix(memory, world_matrix.wrapping_sub(bone * MATRIX_BYTES));
    let [ankle, toe, heel] = read_foot_points(memory, inverse_bind).map(|point| apply(&shin, point));
    let up = normalize(column(&model, 1));
    let floor = column(&model, 3);
    let foot = match turn_onto_floor(ankle, toe, heel, up, floor) {
        Some((axis, angle)) => turned_about(&shin, axis, angle, ankle),
        None => shin,
    };
    write_matrix(memory, skinning_matrix, &foot);
    AfterHook::ReturnToCaller
}

fn read_foot_points(memory: &mut Memory, inverse_bind: u32) -> [Vector; 3] {
    let mut points = [[0.0; 3]; 3];
    for (point_index, point) in points.iter_mut().enumerate() {
        for (axis, value) in point.iter_mut().enumerate() {
            *value = f32::from_bits(memory.read32(inverse_bind + 4 * (3 * point_index + axis) as u32));
        }
    }
    points
}

fn turn_onto_floor(ankle: Vector, toe: Vector, heel: Vector, up: Vector, floor: Vector) -> Option<(Vector, f32)> {
    let height = |point: Vector| dot(up, sub(point, floor));
    if height(toe) >= 0.0 && height(heel) >= 0.0 {
        return None;
    }
    let sole = sub(toe, heel);
    let across = cross(up, sole);
    if dot(across, across) < 1e-12 {
        return None;
    }
    let axis = normalize(across);
    let rise = dot(up, sole);
    let angle_to_flat = (rise / dot(sole, sole).sqrt()).clamp(-1.0, 1.0).asin();
    if rise >= 0.0 {
        return Some((axis, angle_to_flat));
    }
    let toe_height_at = |angle: f32| height(add(ankle, rotate(sub(toe, ankle), axis, angle)));
    if toe_height_at(angle_to_flat) <= 0.0 {
        return Some((axis, angle_to_flat));
    }
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

fn turned_about(matrix: &Matrix, axis: Vector, angle: f32, pivot: Vector) -> Matrix {
    let mut turned = [[0.0; 4]; 3];
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

fn rotate(vector: Vector, axis: Vector, angle: f32) -> Vector {
    let (sin, cos) = angle.sin_cos();
    let across = cross(axis, vector);
    let along = dot(axis, vector) * (1.0 - cos);
    [0, 1, 2].map(|i| vector[i] * cos + across[i] * sin + axis[i] * along)
}

fn column(matrix: &Matrix, index: usize) -> Vector {
    [matrix[0][index], matrix[1][index], matrix[2][index]]
}

fn read_matrix(memory: &mut Memory, address: u32) -> Matrix {
    let mut matrix = [[0.0; 4]; 3];
    for (i, value) in matrix.as_flattened_mut().iter_mut().enumerate() {
        *value = f32::from_bits(memory.read32(address + 4 * i as u32));
    }
    matrix
}

fn write_matrix(memory: &mut Memory, address: u32, matrix: &Matrix) {
    for (i, value) in matrix.as_flattened().iter().enumerate() {
        memory.write32(address + 4 * i as u32, value.to_bits());
    }
}

fn apply(matrix: &Matrix, point: Vector) -> Vector {
    [0, 1, 2].map(|i| matrix[i][0] * point[0] + matrix[i][1] * point[1] + matrix[i][2] * point[2] + matrix[i][3])
}

fn add(a: Vector, b: Vector) -> Vector {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: Vector, b: Vector) -> Vector {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: Vector, b: Vector) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: Vector, b: Vector) -> Vector {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

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

    const UP: Vector = [0.0, 1.0, 0.0];
    const FLOOR: Vector = [0.0, 0.0, 0.0];

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

    #[test]
    fn a_foot_off_the_floor_keeps_its_angle_to_the_shin() {
        let (toe, heel) = stand(15.0, 2.0);
        assert!(toe[1] > 0.0 && heel[1] > 0.0);
        let sin = 15f32.to_radians().sin();
        assert!((toe[1] - (2.0 - 0.8 * 15f32.to_radians().cos() - 1.5 * sin)).abs() < 1e-5, "{toe:?}");
    }

    #[test]
    fn a_toe_that_would_dig_in_comes_up_to_the_floor() {
        let (toe, heel) = stand(15.0, 0.9);
        assert!(toe[1].abs() < 1e-4, "the toe touches, {toe:?}");
        assert!(heel[1] > toe[1], "the heel off the floor, {heel:?}");
    }

    #[test]
    fn a_heel_that_would_dig_in_lays_the_foot_flat() {
        let (toe, heel) = stand(-15.0, 0.85);
        assert!((toe[1] - heel[1]).abs() < 1e-4, "flat, {toe:?} {heel:?}");
        assert!(heel[1] > -0.1 && heel[1] < 0.1, "on the floor, {heel:?}");
    }

    #[test]
    fn a_foot_on_the_floor_lies_flat() {
        let (toe, heel) = stand(15.0, 0.7);
        assert!((toe[1] - heel[1]).abs() < 1e-4, "flat, {toe:?} {heel:?}");
        assert!(toe[1] < 0.0, "under the floor still, the ankle being too low");
    }

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
