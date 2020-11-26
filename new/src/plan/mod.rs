use bevy::prelude::*;
use ndarray::prelude::*;

fn get_all_vertices_and_com(
    origin: &Vec2,
    ls: &Array1<f32>,
    qs: &Array1<f32>,
) -> (Vec<Vec2>, Vec2) {
    let n = ls.len();
    let mut com = Vec2::zero();
    let mut vertices = Vec::with_capacity(ls.len() + 1);
    vertices.push(origin.clone());
    let mut e1 = origin.clone();
    let mut cumulative_rotation = 0f32;
    for i in 0..n {
        cumulative_rotation += qs[i];
        let e2 = e1 + Vec2::new(cumulative_rotation.cos(), cumulative_rotation.sin()) * ls[i];
        vertices.push(e2);
        com += ls[i] * (e1 + e2) / 2.0;
        e1 = e2;
    }
    (vertices, com / ls.sum())
}

fn jacobian_transpose_step(a_i_0: &Vec<Vec2>, goal: &Vec2) -> Array1<f32> {
    let n = a_i_0.len() - 1;
    // Free end coordinates
    let a_e_0 = *a_i_0.last().unwrap();
    // Building jacobian
    let mut jacobian = Array2::zeros((2, n));
    for i in 0..n {
        let a_ie_0 = a_e_0 - a_i_0[i];
        jacobian[(0, i)] = -a_ie_0[1];
        jacobian[(1, i)] = a_ie_0[0];
    }
    // Building delta_x
    let delta_x = arr1(&[goal[0], goal[1]]) - arr1(&[a_e_0[0], a_e_0[1]]);
    // Jacobian transpose
    let delta_q = jacobian.t().dot(&delta_x);
    delta_q
}

fn comx_step(vertices: Vec<Vec2>, comx: f32) -> Array1<f32> {
    // len(vertices) = n + 1
    // Calculate y_1 + y_2 + y_3 + ... y_(n-1) + (y_n / 2); y_0 = 0 anyway so include it for cleaner code
    let sum_y_i = vertices.iter().map(|vertex| vertex[1]).sum::<f32>();
    let last_y_i = vertices.last().unwrap()[1];
    let second_term = sum_y_i - (last_y_i / 2.0);
    // Calculate negative gradient of x_c ^ 2 w.r.t. q_i
    let n = vertices.len() - 1;
    let nf = n as f32;
    let mut delta_q_prev = ((2.0 / nf) * comx) * second_term;
    let mut delta_q = Vec::with_capacity(n);
    delta_q.push(delta_q_prev);
    for i in 1..n {
        // Actual value
        let delta_q_curr =
            delta_q_prev - (2.0 * comx / nf) * vertices[i][1] * (nf - (i as f32) + 0.5);
        // Discounted responsibility for maintainin center of mass over origin
        let delta_q_curr = delta_q_curr / (i as f32);
        delta_q_prev = delta_q_curr;
        delta_q.push(delta_q_prev);
    }
    arr1(&delta_q)
}

pub fn midpoint_comx_ik(
    origin: &Vec2,
    ls: &Array1<f32>,
    qs: &Array1<f32>,
    goal: &Vec2,
) -> (Array1<f32>, Array1<f32>) {
    let (vertices, com) = get_all_vertices_and_com(origin, ls, qs);
    let jt_step = jacobian_transpose_step(&vertices, &goal);
    // Shift origin to first vertex
    let origin = vertices[0];
    let vertices = vertices
        .iter()
        .map(|&vertex| vertex - origin)
        .collect::<Vec<Vec2>>();
    // Set comx goal as (origin_x + goal_x) / 2
    let com = com - ((origin + goal.clone()) / 2.0);
    let comx_step = comx_step(vertices, com[0]);
    (jt_step, comx_step)
}

pub fn origin_comx_ik(
    origin: &Vec2,
    ls: &Array1<f32>,
    qs: &Array1<f32>,
    goal: &Vec2,
) -> (Array1<f32>, Array1<f32>) {
    let (vertices, com) = get_all_vertices_and_com(origin, ls, qs);
    let jt_step = jacobian_transpose_step(&vertices, &goal);
    // Shift origin to first vertex
    let origin = vertices[0];
    let vertices = vertices
        .iter()
        .map(|&vertex| vertex - origin)
        .collect::<Vec<Vec2>>();
    // Set comx goal as origin_x
    let com = com - origin;
    let comx_step = comx_step(vertices, com[0]);
    (jt_step, comx_step)
}
