use font_collector::FontRepository;
use font_rasterizer::{
    color_theme::ColorTheme, context::WindowSize, glyph_instances::GlyphInstances,
    rasterizer_pipeline::Quarity, vector_instances::InstanceAttributes,
};
use glam::Vec3;
use stroke_parser::Action;
use ui_support::ui_context::UiContext;
use ui_support::{
    Flags, InputResult, RenderData, SimpleStateCallback, SimpleStateSupport,
    camera::{Camera, CameraController, CameraOperation},
    easing_value::EasingPointN,
    run_support,
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use web_time::Duration;
use web_time::Instant;
use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
};

const FONT_DATA: &[u8] = include_bytes!("../../../fonts/BIZUDMincho-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("../../../fonts/NotoEmoji-Regular.ttf");

const GRID_HALF_WIDTH: i32 = 8;
const GRID_HALF_HEIGHT: i32 = 6;
const CELL_SIZE: f32 = 1.8;

const JUMP_INITIAL_VELOCITY: f32 = 7.5;
const GRAVITY: f32 = 20.0;
const CAMERA_HEIGHT: f32 = 16.0;
const CAMERA_BACK_OFFSET_Z: f32 = 12.0;
const MOVE_EASING_MILLIS: u64 = 360;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    let mut font_repository = FontRepository::default();
    font_repository.add_fallback_font_from_binary(FONT_DATA.to_vec(), None);
    font_repository.add_fallback_font_from_binary(EMOJI_FONT_DATA.to_vec(), None);

    let window_size = WindowSize::new(1024, 768);
    let callback = PigActionGame::new(window_size);
    let support = SimpleStateSupport {
        window_icon: None,
        window_title: "Pig Action Game".to_string(),
        window_size,
        callback: Box::new(callback),
        quarity: Quarity::High,
        color_theme: ColorTheme::SolarizedDark,
        flags: Flags::DEFAULT,
        font_repository,
        performance_mode: false,
        background_image: None,
        shader_art: None,
    };

    run_support(support).await;
}

struct PigActionGame {
    camera: Camera,
    camera_controller: CameraController,
    pig: Option<GlyphInstances>,
    ground: Option<GlyphInstances>,
    marker: Option<GlyphInstances>,
    grid_x: i32,
    grid_z: i32,
    visual_grid_position: EasingPointN<2>,
    jump_height: f32,
    jump_velocity: f32,
    last_update: Instant,
}

impl PigActionGame {
    fn new(window_size: WindowSize) -> Self {
        let mut camera = Camera::basic(window_size);
        let mut camera_controller = CameraController::new(0.0);
        let mut visual_grid_position: EasingPointN<2> = [0.0, 0.0].into();
        visual_grid_position.update_duration_and_easing_func(
            Duration::from_millis(MOVE_EASING_MILLIS),
            nenobi::functions::sin_in_out,
        );
        camera_controller.process(&CameraOperation::CangeTargetAndEye(
            Vec3::ZERO,
            Vec3::new(0.0, CAMERA_HEIGHT, CAMERA_BACK_OFFSET_Z),
        ));
        camera_controller.update_camera(&mut camera);
        camera_controller.reset_state();

        Self {
            camera,
            camera_controller,
            pig: None,
            ground: None,
            marker: None,
            grid_x: 0,
            grid_z: 0,
            visual_grid_position,
            jump_height: 0.0,
            jump_velocity: 0.0,
            last_update: Instant::now(),
        }
    }

    fn clamp_grid(&mut self) {
        self.grid_x = self.grid_x.clamp(-GRID_HALF_WIDTH, GRID_HALF_WIDTH);
        self.grid_z = self.grid_z.clamp(-GRID_HALF_HEIGHT, GRID_HALF_HEIGHT);
    }

    fn try_move(&mut self, dx: i32, dz: i32) {
        self.grid_x += dx;
        self.grid_z += dz;
        self.clamp_grid();
        self.visual_grid_position.update([
            self.grid_x as f32 * CELL_SIZE,
            self.grid_z as f32 * CELL_SIZE,
        ]);
    }

    fn world_position(&self) -> Vec3 {
        let [x, z] = self.visual_grid_position.current();
        Vec3::new(x, self.jump_height, z)
    }

    fn logical_world_position(&self) -> [f32; 2] {
        [
            self.grid_x as f32 * CELL_SIZE,
            self.grid_z as f32 * CELL_SIZE,
        ]
    }

    fn update_jump(&mut self, dt: f32) {
        if self.jump_height <= 0.0 && self.jump_velocity <= 0.0 {
            self.jump_height = 0.0;
            return;
        }

        self.jump_velocity -= GRAVITY * dt;
        self.jump_height += self.jump_velocity * dt;
        if self.jump_height <= 0.0 {
            self.jump_height = 0.0;
            self.jump_velocity = 0.0;
        }
    }

    fn request_jump(&mut self) {
        if self.jump_height == 0.0 {
            self.jump_velocity = JUMP_INITIAL_VELOCITY;
        }
    }

    fn rebuild_scene_instances(&mut self, context: &UiContext) {
        let pig_position = self.world_position();

        let (Some(ground), Some(marker), Some(pig)) = (
            self.ground.as_mut(),
            self.marker.as_mut(),
            self.pig.as_mut(),
        ) else {
            return;
        };

        ground.clear();
        marker.clear();
        pig.clear();

        let ground_color = context.color_theme().background_highlights().get_color();
        for x in -GRID_HALF_WIDTH..=GRID_HALF_WIDTH {
            for z in -GRID_HALF_HEIGHT..=GRID_HALF_HEIGHT {
                ground.push(InstanceAttributes {
                    position: Vec3::new(x as f32 * CELL_SIZE, -0.3, z as f32 * CELL_SIZE),
                    instance_scale: [0.8, 0.8],
                    color: ground_color,
                    ..Default::default()
                });
            }
        }

        marker.push(InstanceAttributes {
            position: Vec3::new(pig_position.x, -0.05, pig_position.z),
            instance_scale: [0.7, 0.7],
            color: context.color_theme().yellow().get_color(),
            ..Default::default()
        });

        pig.push(InstanceAttributes {
            position: pig_position,
            instance_scale: [1.3, 1.3],
            color: context.color_theme().magenta().get_color(),
            ..Default::default()
        });

        ground.update_buffer(context.device(), context.queue());
        marker.update_buffer(context.device(), context.queue());
        pig.update_buffer(context.device(), context.queue());
    }

    fn update_camera_follow(&mut self) {
        let [x, z] = self.visual_grid_position.current();
        let target = Vec3::new(x, self.jump_height * 0.35, z);
        let eye = target + Vec3::new(0.0, CAMERA_HEIGHT, CAMERA_BACK_OFFSET_Z);
        self.camera_controller
            .process(&CameraOperation::CangeTargetAndEye(target, eye));
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_controller.reset_state();
    }
}

impl SimpleStateCallback for PigActionGame {
    fn init(&mut self, context: &UiContext) {
        self.ground = Some(GlyphInstances::new('・', context.device()));
        self.marker = Some(GlyphInstances::new('+', context.device()));
        self.pig = Some(GlyphInstances::new('🐖', context.device()));
        self.visual_grid_position
            .update(self.logical_world_position());
        context.register_string("🐖・+".to_string());
        self.rebuild_scene_instances(context);
    }

    fn update(&mut self, context: &UiContext) {
        let now = Instant::now();
        let dt = (now - self.last_update).as_secs_f32().min(0.05);
        self.last_update = now;

        self.update_jump(dt);
        self.update_camera_follow();
        self.rebuild_scene_instances(context);
    }

    fn input(&mut self, _context: &UiContext, event: &WindowEvent) -> InputResult {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key,
                        physical_key,
                        repeat,
                        ..
                    },
                ..
            } => {
                if *repeat {
                    return InputResult::InputConsumed;
                }

                if *physical_key == PhysicalKey::Code(KeyCode::Space) {
                    self.request_jump();
                    return InputResult::InputConsumed;
                }

                match logical_key {
                    Key::Named(NamedKey::ArrowUp) => {
                        self.try_move(0, -1);
                        InputResult::InputConsumed
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        self.try_move(0, 1);
                        InputResult::InputConsumed
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        self.try_move(-1, 0);
                        InputResult::InputConsumed
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        self.try_move(1, 0);
                        InputResult::InputConsumed
                    }
                    _ => InputResult::Noop,
                }
            }
            _ => InputResult::Noop,
        }
    }

    fn action(&mut self, _context: &UiContext, _action: Action) -> InputResult {
        InputResult::Noop
    }

    fn resize(&mut self, size: WindowSize) {
        self.camera_controller
            .update_camera_aspect(&mut self.camera, size);
    }

    fn render(&'_ mut self) -> RenderData<'_> {
        let mut glyph_instances = vec![];
        if let Some(ground) = self.ground.as_ref() {
            glyph_instances.push(ground);
        }
        if let Some(marker) = self.marker.as_ref() {
            glyph_instances.push(marker);
        }
        if let Some(pig) = self.pig.as_ref() {
            glyph_instances.push(pig);
        }

        RenderData {
            camera: &self.camera,
            glyph_instances,
            vector_instances: vec![],
            glyph_instances_for_modal: vec![],
            vector_instances_for_modal: vec![],
        }
    }

    fn shutdown(&mut self, _context: &UiContext) {}
}
