//! Bounded original NiAlphaAccumulator behavior for the static scene renderer.
//!
//! 896070 queues geometries; 895ED0 produces x87/f32 keys; 8DA890 performs a
//! nonstable median-of-three sort; 8DA9C0 submits the resulting array backwards.
//! These functions consume caller-supplied world bounds/traversal order. They
//! do not establish the original live scenegraph or floating-point lifecycle.

use aa_numeric::{Binary, Precision};
use anyhow::{Result, ensure};

pub fn queues_geometry(alpha_flags: u16, respect_no_sort: bool) -> bool {
    alpha_flags & 1 != 0 && (!respect_no_sort || alpha_flags & 8192 == 0)
}

/// Camera-direction dot world-sphere center, optionally minus sphere radius.
/// The native operation order is Y product + Z product, then X product. It
/// does not subtract the camera position and does not compute squared distance.
pub fn depth_key(
    center: [f32; 3],
    radius: f32,
    direction: [f32; 3],
    subtract_radius: bool,
    precision: Precision,
) -> Result<f32> {
    let scalar = |value: f32| Binary::from_f32_bits(value.to_bits());
    let product = |axis: usize| scalar(center[axis])?.mul(&scalar(direction[axis])?, precision);
    let mut key = product(1)?
        .add(&product(2)?, precision)?
        .add(&product(0)?, precision)?;
    if subtract_radius {
        key = key.sub(&scalar(radius)?, precision)?;
    }
    Ok(f32::from_bits(key.store_f32()?))
}

#[derive(Clone, Copy, Debug)]
pub struct Entry<T> {
    pub key: f32,
    pub value: T,
}

fn pivot<T>(items: &[Entry<T>], first: usize, last: usize) -> f32 {
    let a = items[first].key;
    let b = items[(first + last) / 2].key;
    let c = items[last].key;
    if a >= b {
        if a < c {
            a
        } else if b >= c {
            b
        } else {
            c
        }
    } else if b < c {
        b
    } else if a >= c {
        a
    } else {
        c
    }
}

/// Reproduce the native unstable tie order, including equal-depth surfaces.
/// Non-finite keys are rejected; they are not part of the supported asset path.
/// An explicit work stack avoids the original recursive call-stack limitation.
pub fn submission_order<T>(items: &mut [Entry<T>]) -> Result<()> {
    ensure!(
        items.iter().all(|entry| entry.key.is_finite()),
        "Non-finite transparent sort key"
    );
    if items.len() < 2 {
        return Ok(());
    }
    let mut ranges = vec![(0, items.len() - 1)];
    while let Some((first, mut last)) = ranges.pop() {
        while first < last {
            let middle = pivot(items, first, last);
            let mut left = first;
            let mut right = last;
            loop {
                while items[right].key > middle {
                    right -= 1;
                }
                while items[left].key < middle {
                    left += 1;
                }
                if left >= right {
                    break;
                }
                items.swap(left, right);
                left += 1;
                right -= 1;
            }
            if right == last {
                last -= 1;
            } else {
                ranges.push((right + 1, last));
                last = right;
            }
        }
    }
    items.reverse();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Vectors {
        policy: Vec<Policy>,
        sort: Vec<Sort>,
    }
    #[derive(Deserialize)]
    struct Policy {
        flags: u16,
        respect_no_sort: bool,
        queued: bool,
    }
    #[derive(Deserialize)]
    struct Sort {
        precision: Precision,
        centers: Vec<[f32; 3]>,
        radii: Vec<f32>,
        direction: [f32; 3],
        subtract_radius: bool,
        key_bits: Vec<u32>,
        draw_order: Vec<usize>,
    }
    #[test]
    fn original_accumulator_keys_ties_and_submission_match() {
        let fixtures: Vectors = serde_json::from_str(include_str!(
            "../../../research/phase6-material/native-transparency-vectors.json"
        ))
        .unwrap();
        for case in fixtures.policy {
            assert_eq!(
                queues_geometry(case.flags, case.respect_no_sort),
                case.queued
            );
        }
        for (case_index, case) in fixtures.sort.iter().enumerate() {
            let mut entries = Vec::new();
            for (index, center) in case.centers.iter().enumerate() {
                let key = depth_key(
                    *center,
                    case.radii[index],
                    case.direction,
                    case.subtract_radius,
                    case.precision,
                )
                .unwrap();
                assert_eq!(
                    key.to_bits(),
                    case.key_bits[index],
                    "case {case_index}, key {index}"
                );
                entries.push(Entry { key, value: index });
            }
            submission_order(&mut entries).unwrap();
            assert_eq!(
                entries.iter().map(|row| row.value).collect::<Vec<_>>(),
                case.draw_order,
                "native submission/tie order case {case_index}"
            );
        }
    }

    #[test]
    fn invalid_bounds_do_not_enter_native_partition_loops() {
        assert!(
            depth_key(
                [f32::NAN, 0.0, 0.0],
                0.0,
                [1.0; 3],
                false,
                Precision::Single24
            )
            .is_err()
        );
        let mut invalid = [Entry {
            key: f32::INFINITY,
            value: 0,
        }];
        assert!(submission_order(&mut invalid).is_err());
    }
}
