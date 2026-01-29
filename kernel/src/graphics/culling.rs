//! View Frustum Culling Module
//!
//! Provides efficient culling of objects outside the camera's view frustum.
//! This significantly reduces the number of triangles that need to be transformed
//! and rasterized.

use glam::{Mat4, Vec3, Vec4};
use libm::{ceilf, sqrtf};

/// Axis-Aligned Bounding Box for fast culling tests
#[derive(Clone, Copy, Debug)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Create AABB from center and half-extents
    pub fn from_center_extents(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    /// Get the center of the AABB
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Get the half-extents (size/2)
    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    /// Transform AABB by a matrix (returns conservative AABB)
    pub fn transform(&self, matrix: &Mat4) -> Self {
        let center = self.center();
        let extents = self.half_extents();

        // Transform center
        let new_center = matrix.transform_point3(center);

        // Transform extents (use absolute values of matrix for conservative bounds)
        let abs_mat = Mat4::from_cols(
            matrix.x_axis.abs(),
            matrix.y_axis.abs(),
            matrix.z_axis.abs(),
            Vec4::W,
        );
        let new_extents = abs_mat.transform_vector3(extents);

        Self::from_center_extents(new_center, new_extents)
    }
}

/// View frustum for culling
pub struct Frustum {
    /// Frustum planes in world space (normal pointing inward)
    /// Order: left, right, bottom, top, near, far
    planes: [Vec4; 6],
}

impl Frustum {
    /// Extract frustum planes from view-projection matrix
    /// Uses Gribb-Hartmann method for fast extraction
    pub fn from_view_projection(vp: &Mat4) -> Self {
        let row0 = Vec4::new(vp.x_axis.x, vp.y_axis.x, vp.z_axis.x, vp.w_axis.x);
        let row1 = Vec4::new(vp.x_axis.y, vp.y_axis.y, vp.z_axis.y, vp.w_axis.y);
        let row2 = Vec4::new(vp.x_axis.z, vp.y_axis.z, vp.z_axis.z, vp.w_axis.z);
        let row3 = Vec4::new(vp.x_axis.w, vp.y_axis.w, vp.z_axis.w, vp.w_axis.w);

        // Extract and normalize planes
        let mut planes = [
            row3 + row0, // Left
            row3 - row0, // Right
            row3 + row1, // Bottom
            row3 - row1, // Top
            row3 + row2, // Near
            row3 - row2, // Far
        ];

        // Normalize all planes
        for plane in &mut planes {
            let len = sqrtf(plane.x * plane.x + plane.y * plane.y + plane.z * plane.z);
            if len > 0.0001 {
                *plane /= len;
            }
        }

        Self { planes }
    }

    /// Test if a point is inside the frustum
    pub fn contains_point(&self, point: Vec3) -> bool {
        for plane in &self.planes {
            let dist = plane.x * point.x + plane.y * point.y + plane.z * point.z + plane.w;
            if dist < 0.0 {
                return false;
            }
        }
        true
    }

    /// Test if an AABB intersects the frustum
    /// Returns true if the AABB is fully or partially inside
    pub fn intersects_aabb(&self, aabb: &AABB) -> bool {
        for plane in &self.planes {
            // Find the point of the AABB most in the direction of the plane normal
            let p = Vec3::new(
                if plane.x >= 0.0 { aabb.max.x } else { aabb.min.x },
                if plane.y >= 0.0 { aabb.max.y } else { aabb.min.y },
                if plane.z >= 0.0 { aabb.max.z } else { aabb.min.z },
            );

            // Test if this point is outside the plane
            let dist = plane.x * p.x + plane.y * p.y + plane.z * p.z + plane.w;
            if dist < 0.0 {
                return false; // Completely outside this plane
            }
        }
        true
    }

    /// Test if a sphere intersects the frustum
    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            let dist = plane.x * center.x + plane.y * center.y + plane.z * center.z + plane.w;
            if dist < -radius {
                return false; // Completely outside this plane
            }
        }
        true
    }
}

/// Quick distance-based culling check
#[inline]
pub fn distance_cull(object_pos: Vec3, camera_pos: Vec3, max_distance: f32) -> bool {
    let dx = object_pos.x - camera_pos.x;
    let dz = object_pos.z - camera_pos.z;
    dx * dx + dz * dz > max_distance * max_distance
}

/// Combined frustum + distance culling for efficiency
pub struct CullContext {
    pub frustum: Frustum,
    pub camera_pos: Vec3,
    pub near_cull_distance: f32,  // Don't render objects too close (inside player)
    pub far_cull_distance: f32,   // Don't render objects too far
}

impl CullContext {
    pub fn new(view: &Mat4, projection: &Mat4, camera_pos: Vec3) -> Self {
        let vp = *projection * *view;
        Self {
            frustum: Frustum::from_view_projection(&vp),
            camera_pos,
            near_cull_distance: 0.5,
            far_cull_distance: 500.0,
        }
    }

    /// Set custom cull distances
    pub fn with_distances(mut self, near: f32, far: f32) -> Self {
        self.near_cull_distance = near;
        self.far_cull_distance = far;
        self
    }

    /// Test if an object at position with bounding radius should be rendered
    /// Frustum culling DISABLED - was causing objects to disappear incorrectly
    /// Only uses simple distance culling for performance
    pub fn should_render(&self, position: Vec3, radius: f32) -> bool {
        // Simple 3D distance culling only (frustum culling disabled)
        let dx = position.x - self.camera_pos.x;
        let dy = position.y - self.camera_pos.y;
        let dz = position.z - self.camera_pos.z;
        let dist_sq = dx * dx + dy * dy + dz * dz;

        // Far distance culling with generous tolerance
        let effective_far = self.far_cull_distance + radius + 100.0;
        let far_sq = effective_far * effective_far;

        dist_sq <= far_sq
    }

    /// Test if an AABB should be rendered
    /// Frustum culling DISABLED - was causing objects to disappear incorrectly
    pub fn should_render_aabb(&self, aabb: &AABB) -> bool {
        // Distance culling using AABB center
        let center = aabb.center();
        let radius = aabb.half_extents().length();

        let dx = center.x - self.camera_pos.x;
        let dy = center.y - self.camera_pos.y;
        let dz = center.z - self.camera_pos.z;
        let dist_sq = dx * dx + dy * dy + dz * dz;

        // Generous far distance
        let effective_far = self.far_cull_distance + radius + 100.0;
        let far_sq = effective_far * effective_far;

        dist_sq <= far_sq
    }
}

/// Terrain chunk for efficient culling
/// Instead of one 20k triangle mesh, split into smaller chunks
pub const TERRAIN_CHUNK_SIZE: f32 = 100.0; // 100x100 unit chunks
pub const TERRAIN_CHUNK_SUBDIVISIONS: usize = 5; // 5x5 grid per chunk = 50 triangles each

/// Calculate which terrain chunks are visible
pub fn get_visible_terrain_chunks(
    cull_ctx: &CullContext,
    terrain_size: f32,
    chunk_size: f32,
) -> impl Iterator<Item = (i32, i32)> + '_ {
    let chunks_per_side = ceilf(terrain_size / chunk_size) as i32;
    let half_chunks = chunks_per_side / 2;

    (-half_chunks..=half_chunks).flat_map(move |cz| {
        (-half_chunks..=half_chunks).filter_map(move |cx| {
            // Calculate chunk AABB
            let min_x = cx as f32 * chunk_size;
            let min_z = cz as f32 * chunk_size;
            let max_x = min_x + chunk_size;
            let max_z = min_z + chunk_size;

            let aabb = AABB::new(
                Vec3::new(min_x, -10.0, min_z), // Allow for terrain height variation
                Vec3::new(max_x, 10.0, max_z),
            );

            if cull_ctx.should_render_aabb(&aabb) {
                Some((cx, cz))
            } else {
                None
            }
        })
    })
}
