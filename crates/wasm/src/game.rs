//! The 3D board: renders the 4x4x4 cube with three-d on WebGL2 and turns clicks into
//! moves on a `patches_core::Board`. JavaScript owns the canvas events and forwards them.

use std::sync::Arc;

use patches_core::{Board, BoxRegion, CELLS, Cell, N, Puzzle, cell_at};
use three_d::*;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

const CELL_SIZE: f32 = 0.72;
/// Vertical distance between layers, in cell units, so every layer's top faces stay visible.
const LAYER_PITCH: f32 = 2.5;
const CAMERA_DISTANCE: f32 = 24.0;
/// Pointer travel (CSS px) below which a press counts as a click, not an orbit drag.
const CLICK_SLOP: f32 = 5.0;
const ORBIT_SPEED: f32 = 0.006;
const PITCH_RANGE: (f32, f32) = (0.15, 1.45);

const BASE: Srgba = Srgba::new_opaque(0xe4, 0xe4, 0xe7);
const PENDING: Srgba = Srgba::new_opaque(0x52, 0x52, 0x5b);
/// Same colours as `boxColor` in web/src/lib/puzzle.ts so the 3D view matches the layer grids.
const PALETTE: [Srgba; 12] = [
    Srgba::new_opaque(0xf1, 0xb1, 0xb1),
    Srgba::new_opaque(0xb1, 0xf1, 0xc4),
    Srgba::new_opaque(0xd6, 0xb1, 0xf1),
    Srgba::new_opaque(0xf1, 0xe9, 0xb1),
    Srgba::new_opaque(0xb1, 0xe6, 0xf1),
    Srgba::new_opaque(0xf1, 0xb1, 0xd4),
    Srgba::new_opaque(0xc1, 0xf1, 0xb1),
    Srgba::new_opaque(0xb4, 0xb1, 0xf1),
    Srgba::new_opaque(0xf1, 0xc6, 0xb1),
    Srgba::new_opaque(0xb1, 0xf1, 0xd9),
    Srgba::new_opaque(0xec, 0xb1, 0xf1),
    Srgba::new_opaque(0xe4, 0xf1, 0xb1),
];

struct Drag {
    start: (f32, f32),
    last: (f32, f32),
    moved: bool,
}

#[wasm_bindgen]
pub struct Game {
    canvas: HtmlCanvasElement,
    context: Context,
    camera: Camera,
    yaw: f32,
    pitch: f32,
    cells: Gm<InstancedMesh, PhysicalMaterial>,
    transforms: Vec<Mat4>,
    ambient: AmbientLight,
    sun: DirectionalLight,
    puzzle: Puzzle,
    board: Board,
    pending: Option<Cell>,
    drag: Option<Drag>,
    interactive: bool,
}

#[wasm_bindgen]
impl Game {
    /// `answer = true` shows the stored solution and ignores input.
    // three-d wants an Arc'd context; WebGL is single-threaded, so Send/Sync never matters here.
    #[allow(clippy::arc_with_non_send_sync)]
    #[wasm_bindgen(constructor)]
    pub fn new(
        canvas: HtmlCanvasElement,
        puzzle_json: &str,
        answer: bool,
    ) -> Result<Game, JsValue> {
        console_error_panic_hook::set_once();
        let puzzle: Puzzle = serde_json::from_str(puzzle_json).map_err(err)?;
        let gl = canvas
            .get_context("webgl2")?
            .ok_or("WebGL2 is not available")?
            .dyn_into::<web_sys::WebGl2RenderingContext>()?;
        let context =
            Context::from_gl_context(Arc::new(three_d::context::Context::from_webgl2_context(gl)))
                .map_err(err)?;

        let camera = Camera::new_perspective(
            Viewport::new_at_origo(canvas.width(), canvas.height()),
            vec3(0.0, 0.0, CAMERA_DISTANCE),
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            Deg(35.0),
            0.1,
            100.0,
        );
        let transforms: Vec<Mat4> = (0..CELLS)
            .map(|i| {
                Mat4::from_translation(cell_center(cell_at(i))) * Mat4::from_scale(CELL_SIZE / 2.0)
            })
            .collect();
        let mesh = InstancedMesh::new(
            &context,
            &Instances {
                transformations: transforms.clone(),
                colors: Some(vec![BASE; CELLS]),
                ..Default::default()
            },
            &CpuMesh::cube(),
        );
        let material = PhysicalMaterial::new_opaque(
            &context,
            &CpuMaterial {
                albedo: Srgba::WHITE,
                roughness: 0.8,
                metallic: 0.0,
                ..Default::default()
            },
        );
        let mut board = Board::new(puzzle.clues.clone());
        if answer {
            for b in &puzzle.solution {
                board
                    .place(*b)
                    .map_err(|e| format!("stored solution breaks the rules: {e:?}"))?;
            }
        }
        let mut game = Game {
            canvas,
            camera,
            yaw: 0.7,
            pitch: 0.6,
            cells: Gm::new(mesh, material),
            transforms,
            ambient: AmbientLight::new(&context, 0.6, Srgba::WHITE),
            sun: DirectionalLight::new(&context, 1.2, Srgba::WHITE, vec3(-0.5, -1.0, -0.7)),
            context,
            puzzle,
            board,
            pending: None,
            drag: None,
            interactive: !answer,
        };
        game.update_camera();
        game.update_colors();
        Ok(game)
    }

    pub fn render(&mut self) {
        let (w, h) = (self.canvas.width(), self.canvas.height());
        self.camera.set_viewport(Viewport::new_at_origo(w, h));
        RenderTarget::screen(&self.context, w, h)
            .clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 0.0, 1.0))
            .render(&self.camera, [&self.cells], &[&self.ambient, &self.sun]);
    }

    /// Screen position (CSS px, top-left origin) of every clue's top face, flattened as `[x0, y0, x1, y1, ...]`.
    pub fn labels(&self) -> Vec<f32> {
        let s = self.scale();
        let h = self.canvas.height() as f32;
        self.puzzle
            .clues
            .iter()
            .flat_map(|c| {
                let top = cell_center(c.cell) + vec3(0.0, CELL_SIZE / 2.0, 0.0);
                let p = self.camera.pixel_at_position(top);
                [p.x / s, (h - p.y) / s]
            })
            .collect()
    }

    pub fn pointer_down(&mut self, x: f32, y: f32) {
        self.drag = Some(Drag {
            start: (x, y),
            last: (x, y),
            moved: false,
        });
    }

    /// Returns true when the camera moved and the scene needs a redraw.
    pub fn pointer_move(&mut self, x: f32, y: f32) -> bool {
        let Some(d) = self.drag.as_mut() else {
            return false;
        };
        let (dx, dy) = (x - d.last.0, y - d.last.1);
        d.last = (x, y);
        d.moved |= (x - d.start.0).hypot(y - d.start.1) >= CLICK_SLOP;
        if !d.moved {
            return false;
        }
        self.yaw -= dx * ORBIT_SPEED;
        self.pitch = (self.pitch + dy * ORBIT_SPEED).clamp(PITCH_RANGE.0, PITCH_RANGE.1);
        self.update_camera();
        true
    }

    /// Returns true when the board changed.
    pub fn pointer_up(&mut self, x: f32, y: f32) -> bool {
        let Some(d) = self.drag.take() else {
            return false;
        };
        !d.moved && self.click(x, y)
    }

    pub fn box_count(&self) -> u32 {
        self.board.boxes().len() as u32
    }

    pub fn is_solved(&self) -> bool {
        self.board.is_solved()
    }

    pub fn reset(&mut self) {
        if self.interactive {
            self.board.clear();
            self.pending = None;
            self.update_colors();
        }
    }
}

impl Game {
    fn click(&mut self, x: f32, y: f32) -> bool {
        if !self.interactive {
            return false;
        }
        match self.pick_cell(x, y) {
            Some(cell) if self.board.box_at(cell).is_some() => {
                self.board.remove_at(cell);
            }
            Some(cell) => match self.pending.take() {
                None => self.pending = Some(cell),
                // A box that breaks the rules is simply not placed; the selection clears either way.
                Some(first) => drop(self.board.place(BoxRegion::spanning(first, cell))),
            },
            None => self.pending = None,
        }
        self.update_colors();
        true
    }

    fn pick_cell(&self, x: f32, y: f32) -> Option<Cell> {
        let s = self.scale();
        let pixel = (x * s, self.canvas.height() as f32 - y * s);
        let hit = pick(
            &self.context,
            &self.camera,
            pixel,
            [&self.cells],
            Cull::Back,
        )
        .ok()
        .flatten()?;
        Some(cell_at(hit.instance_id as usize))
    }

    /// Physical pixels per CSS pixel of the canvas.
    fn scale(&self) -> f32 {
        self.canvas.width() as f32 / self.canvas.client_width().max(1) as f32
    }

    fn update_camera(&mut self) {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let position = vec3(cp * sy, sp, cp * cy) * CAMERA_DISTANCE;
        self.camera
            .set_view(position, vec3(0.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0));
    }

    fn update_colors(&mut self) {
        let colors = (0..CELLS)
            .map(|i| {
                let cell = cell_at(i);
                match self.board.box_at(cell) {
                    Some(b) => PALETTE[b % PALETTE.len()],
                    None if self.pending == Some(cell) => PENDING,
                    None => BASE,
                }
            })
            .collect();
        self.cells.set_instances(&Instances {
            transformations: self.transforms.clone(),
            colors: Some(colors),
            ..Default::default()
        });
    }
}

/// World position of a cell's centre: x and z on a unit grid, y stretched by `LAYER_PITCH`.
fn cell_center(c: Cell) -> Vec3 {
    let mid = (N as f32 - 1.0) / 2.0;
    vec3(
        c[0] as f32 - mid,
        (c[1] as f32 - mid) * LAYER_PITCH,
        c[2] as f32 - mid,
    )
}

fn err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}
