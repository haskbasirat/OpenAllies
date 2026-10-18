use glam::{Mat4, Vec3};

/// Render camera only. It does not advance or mutate authoritative game state.
pub struct Camera {
    pub target: Vec3,
    pub zoom: f32,
    pub yaw: f32,
    pub map_size: [f32; 2],
    pub inspection: bool,
    height_scale: f32,
    model_radius: f32,
}
impl Camera {
    pub fn new(size: [f32; 2]) -> Self {
        Self {
            target: Vec3::new(size[0] * 0.5, 0.0, -size[1] * 0.5),
            zoom: 2.1,
            yaw: -std::f32::consts::FRAC_PI_4,
            map_size: size,
            inspection: false,
            height_scale: 100.0,
            model_radius: 0.0,
        }
    }
    pub fn for_model(bounds: &aa_assets::Bounds) -> Self {
        let extent = Vec3::from(bounds.max) - Vec3::from(bounds.min);
        Self {
            target: (Vec3::from(bounds.min) + Vec3::from(bounds.max)) * 0.5,
            zoom: 1.0,
            yaw: -std::f32::consts::FRAC_PI_4,
            map_size: [extent.x, extent.z],
            inspection: true,
            height_scale: extent.max_element().max(0.1) * 1.6,
            model_radius: extent.length() * 0.5,
        }
    }
    pub fn eye(&self) -> Vec3 {
        let height = self.zoom * self.height_scale;
        let horizontal = height / 40.0_f32.to_radians().tan();
        self.target
            + Vec3::new(
                horizontal * self.yaw.cos(),
                height,
                -horizontal * self.yaw.sin(),
            )
    }
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        // Projection parity is not yet established; this is the asset-inspection camera.
        let far = if self.inspection {
            self.eye().distance(self.target) + self.model_radius.max(0.1) * 3.0
        } else {
            self.map_size[0].max(self.map_size[1]) * 4.0 + 1000.0
        };
        let near = if self.inspection {
            self.height_scale * 0.002
        } else {
            0.5
        };
        Mat4::perspective_rh(43.0_f32.to_radians(), aspect.max(0.01), near, far)
            * Mat4::look_at_rh(self.eye(), self.target, Vec3::Y)
    }
    pub fn pan(&mut self, horizontal: f32, vertical: f32, seconds: f32) {
        let forward = (self.target - self.eye()).with_y(0.0).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let direction = right * horizontal + forward * vertical;
        if direction.length_squared() > 0.0 {
            let speed = if self.inspection {
                0.65 * self.height_scale
            } else {
                65.0
            };
            self.target += direction.normalize() * seconds * speed * self.zoom;
            if !self.inspection {
                self.target.x = self.target.x.clamp(0.0, self.map_size[0]);
                self.target.z = self.target.z.clamp(-self.map_size[1], 0.0);
            }
        }
    }
    pub fn zoom_by(&mut self, delta: f32) {
        let limits = if self.inspection {
            [0.2, 8.0]
        } else {
            [0.7, 2.1]
        };
        self.zoom = (self.zoom * (-delta * 0.002).exp()).clamp(limits[0], limits[1]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn camera_height_matches_recorded_expanded_native_endpoints() {
        let mut c = Camera::new([1024.0, 1024.0]);
        c.target = Vec3::new(239.0, 3.0932593, -699.0);
        let eye = c.eye();
        assert!((eye.x - 415.96637).abs() < 0.001);
        assert!((eye.y - 213.09323).abs() < 0.001);
        assert!((eye.z + 522.033_6).abs() < 0.001);
        c.zoom_by(100000.0);
        assert_eq!(c.zoom, 0.7);
        c.zoom_by(-100000.0);
        assert_eq!(c.zoom, 2.1);
    }
    #[test]
    fn tall_and_wide_inspector_bounds_are_not_clipped_at_far_zoom() {
        for size in [[12.4042, 2858.8318, 11.2502], [600.0, 20.0, 500.0]] {
            let bounds = aa_assets::Bounds {
                min: [0.0; 3],
                max: size,
                size,
            };
            let mut camera = Camera::for_model(&bounds);
            camera.zoom = 8.0;
            for x in [0.0, size[0]] {
                for y in [0.0, size[1]] {
                    for z in [0.0, size[2]] {
                        let clip =
                            camera.view_projection(16.0 / 9.0) * Vec3::new(x, y, z).extend(1.0);
                        assert!(
                            clip.z >= 0.0 && clip.z <= clip.w,
                            "model bounds escaped depth range: {clip:?}"
                        );
                    }
                }
            }
        }
    }
}
