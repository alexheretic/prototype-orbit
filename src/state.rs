use cgmath::*;
use OrbitBody;
use orbitcurve::OrbitCurve;
use debug::ComputeDebugInfo;
use uuid::Uuid;
use std::f64;

#[derive(Clone, Debug)]
pub struct Drawables {
    pub orbit_bodies: Vec<OrbitBody>,
    pub orbit_curves: Vec<OrbitCurve>,
}

impl Drawables {
    fn initial() -> Drawables {
        let bodies = vec!(
            OrbitBody {
                id: Uuid::new_v4(),
                center: (0.0, 0.0).into(),
                radius: 1.2,
                mass: 1660.0,
                velocity: (0.0, -1.0).into(),
            },
            OrbitBody {
                id: Uuid::new_v4(),
                center: (3.5, 0.0).into(),
                radius: 0.9,
                mass: 1000.0,
                velocity: (0.0, 1.6).into(),
            },
            OrbitBody {
                id: Uuid::new_v4(),
                center: (9.0, 0.0).into(),
                radius: 0.3,
                mass: 1.0,
                velocity: (0.0, 2.0).into(),
            },
            OrbitBody {
                id: Uuid::new_v4(),
                center: (-12.0, 0.0).into(),
                radius: 0.4,
                mass: 2.0,
                velocity: (0.0, -1.5).into(),
            },
        );

        Drawables {
            orbit_bodies: bodies,
            orbit_curves: Vec::new(),
        }
    }

    /// :fault_tolerance fraction of smallest body radius error tolerance, in range (0,1]
    pub fn curve_body_mismatch(&self, fault_tolerance: f64) -> bool {
        let mismatch_distance = self.orbit_bodies.iter()
            .map(|b| b.radius)
            .fold(1./0., f64::min) * fault_tolerance;

        for (idx, curve) in self.orbit_curves.iter().enumerate() {
            if !curve.plots.is_empty() {
                let dist_to_body = curve.plots[0].distance(self.orbit_bodies[idx].center);
                if dist_to_body > mismatch_distance {
                    return true;
                }
            }
        }
        false
    }
}

fn birds_eye_at_z(height: f32) -> Matrix4<f32> {
    let mut view = Matrix4::identity();
    view.z.z = height;
    view
}

#[derive(Clone, Debug)]
pub struct State {
    pub origin: Vector2<f32>,
    pub zoom: f32,
    pub screen_width: u32,
    pub screen_height: u32,
    pub view: Matrix4<f32>,
    pub user_quit: bool,
    pub drawables: Drawables,
    pub debug_info: ComputeDebugInfo,
    pub pause: bool,
    pub render_curves: bool,
}

impl State {
    pub fn new(screen_width: u32, screen_height: u32) -> State {
        State {
            origin: Vector2::new(0.0f32, 0.0),
            zoom: 16f32,
            screen_width,
            screen_height,
            view: birds_eye_at_z(1.0),
            user_quit: false,
            drawables: Drawables::initial(),
            debug_info: ComputeDebugInfo::initial(),
            pause: false,
            render_curves: true,
        }
    }

    pub fn projection(&self) -> Matrix4<f32> {
        ortho(self.origin.x - self.zoom * self.aspect_ratio(),
              self.origin.x + self.zoom * self.aspect_ratio(),
              self.origin.y - self.zoom,
              self.origin.y + self.zoom,
              1.0,
              -1.0)
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.screen_width as f32 / self.screen_height as f32
    }

    /// translates screen pixels into world co-ordinates in the orthographic projection
    pub fn screen_to_world_normalised<V: Into<Vector2<i32>>>(&self, pixels: V) -> Vector2<f32> {
        let pixels = pixels.into();
        let x_world = self.zoom * self.aspect_ratio() * (pixels.x as f32 * 2.0 / self.screen_width as f32 - 1f32);
        let y_world = self.zoom * (-pixels.y as f32 * 2.0 / self.screen_height as f32 + 1f32);
        Vector2::new(x_world, y_world)
    }

    pub fn screen_to_world<V: Into<Vector2<i32>>>(&self, pixels: V) -> Vector2<f32> {
        self.origin + self.screen_to_world_normalised(pixels)
    }

    /// Returns tuple with (min, max) coord corners
    /// - left: bottom left, least x & y visible world location
    /// - right: top right, most x & y visible world location
    pub fn visible_world_range(&self) -> (Vector2<f32>, Vector2<f32>) {
        (self.screen_to_world(Vector2::new(0, self.screen_height as i32)),
         self.screen_to_world(Vector2::new(self.screen_width as i32, 0)))
    }
}

#[cfg(test)]
mod state_test {
    use super::*;

    // see https://github.com/gfx-rs/gfx/tree/master/src/backend/gl
    // (0,0)
    //     ┌─┐
    //     └─┘
    //        (width-px, height-px)
    //      |
    //      v
    // (-1az,1z)
    //     ┌─┐
    //     └─┘
    //        (1az,-1z)
    // :a aspect ratio
    // :z zoom
    fn test_screen_to_world(s: State) {
        let a = s.aspect_ratio();
        let z = s.zoom;
        assert_eq!(s.screen_to_world(Vector2::new(0, 0)),
            Vector2::new(-a * z, 1f32 * z), "top-left");
        assert_eq!(s.screen_to_world(Vector2::new(s.screen_width as i32, 0)),
            Vector2::new(a * z, 1f32 * z), "top-right");
        assert_eq!(s.screen_to_world(Vector2::new(0, s.screen_height as i32)),
            Vector2::new(-a * z, -1f32 * z), "bottom-left");
        assert_eq!(s.screen_to_world(Vector2::new(s.screen_width as i32, s.screen_height as i32)),
            Vector2::new(a * z, -1f32 * z), "bottom-right");
        assert_eq!(s.screen_to_world(Vector2::new(s.screen_width as i32 / 2, s.screen_height as i32 / 2 )),
            Vector2::new(0f32, 0f32), "center");
    }
    #[test]
    fn screen_to_world_zoom_1_aspect_1() {
        //      | simplifies to
        //      v
        // (-1,1)
        //     ┌─┐
        //     └─┘
        //        (1,-1)
        test_screen_to_world(State::new(100, 100));
    }

    #[test]
    fn screen_to_world_zoom_1() {
        //      | simplifies to
        //      v
        // (-1a,1)
        //     ┌─┐
        //     └─┘
        //        (1a,-1)
        test_screen_to_world(State::new(160, 90));
    }

    #[test]
    fn screen_to_world() {
        //      | simplifies to
        //      v
        // (-1a,1)
        //     ┌─┐
        //     └─┘
        //        (1a,-1)
        let mut state = State::new(160, 90);
        state.zoom = 0.33f32;
        test_screen_to_world(state);
    }

    #[test]
    fn visible_world_range() {
        let mut state = State::new(180, 90);
        state.zoom = 3f32;
        assert_eq!(state.visible_world_range(), ((-6_f32, -3_f32).into(), (6_f32, 3_f32).into()));
    }
}
