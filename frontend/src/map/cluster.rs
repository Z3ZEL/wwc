//! Marker clustering in screen space: markers closer than a distance are drawn as one
//! cluster with a count. Greedy and O(n²), which is fine for one page of markers
//! (`api::MARKERS_PER_REQUEST`). Only relative positions matter, so panning never
//! changes the clusters; zooming does.

use egui::Pos2;

#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    /// Mean screen position of the members.
    pub center: Pos2,
    /// Indices into the input slice, in input order. Never empty.
    pub members: Vec<usize>,
}

/// Groups `points` so that every member is within `distance` of its cluster's first
/// member. Deterministic: the same input gives the same clusters. `distance <= 0` puts
/// every point in its own cluster.
pub fn cluster(points: &[Pos2], distance: f32) -> Vec<Cluster> {
    let mut taken = vec![false; points.len()];
    let mut out = Vec::new();
    for (i, &seed) in points.iter().enumerate() {
        if taken[i] {
            continue;
        }
        let mut members = vec![i];
        taken[i] = true;
        if distance > 0.0 {
            for (j, &p) in points.iter().enumerate().skip(i + 1) {
                if !taken[j] && seed.distance(p) <= distance {
                    taken[j] = true;
                    members.push(j);
                }
            }
        }
        let sum = members.iter().fold(egui::Vec2::ZERO, |acc, &m| acc + points[m].to_vec2());
        out.push(Cluster { center: (sum / members.len() as f32).to_pos2(), members });
    }
    out
}

/// Radius of a cluster marker: `min` for 2 members, growing with log2 of the count up to `max`.
pub fn cluster_radius(count: usize, min: f32, max: f32) -> f32 {
    let extra = (count.max(2) as f32).log2() - 1.0;
    (min + extra * (max - min) / 6.0).clamp(min, max.max(min))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> Pos2 {
        Pos2::new(x, y)
    }

    #[test]
    fn close_points_share_a_cluster() {
        let pts = [p(0.0, 0.0), p(100.0, 0.0), p(10.0, 0.0), p(105.0, 5.0), p(300.0, 300.0)];
        let c = cluster(&pts, 20.0);
        assert_eq!(c.len(), 3);
        assert_eq!(c[0].members, [0, 2]);
        assert_eq!(c[0].center, p(5.0, 0.0));
        assert_eq!(c[1].members, [1, 3]);
        assert_eq!(c[2].members, [4]);
        assert_eq!(c[2].center, p(300.0, 300.0));
    }

    #[test]
    fn every_point_is_in_exactly_one_cluster() {
        let pts: Vec<Pos2> = (0..50).map(|i| p((i * 7 % 60) as f32, (i * 13 % 45) as f32)).collect();
        let mut seen: Vec<usize> = cluster(&pts, 15.0).into_iter().flat_map(|c| c.members).collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..50).collect::<Vec<_>>());
    }

    #[test]
    fn panning_does_not_change_clusters() {
        let pts = [p(0.0, 0.0), p(30.0, 0.0), p(60.0, 0.0), p(200.0, 10.0)];
        let moved: Vec<Pos2> = pts.iter().map(|q| *q + egui::Vec2::new(-512.3, 77.7)).collect();
        let members = |v: Vec<Cluster>| v.into_iter().map(|c| c.members).collect::<Vec<_>>();
        assert_eq!(members(cluster(&pts, 35.0)), members(cluster(&moved, 35.0)));
    }

    #[test]
    fn zero_distance_disables_clustering() {
        let pts = [p(1.0, 1.0), p(1.0, 1.0)];
        assert_eq!(cluster(&pts, 0.0).len(), 2);
        assert!(cluster(&[], 10.0).is_empty());
    }

    #[test]
    fn radius_grows_with_count_and_is_capped() {
        assert_eq!(cluster_radius(2, 13.0, 22.0), 13.0);
        assert!(cluster_radius(10, 13.0, 22.0) > cluster_radius(3, 13.0, 22.0));
        assert_eq!(cluster_radius(100_000, 13.0, 22.0), 22.0);
    }
}
