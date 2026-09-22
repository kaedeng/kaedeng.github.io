//! The 3D board: a white wireframe of the 4x4x4 cube, rendered with three-d on WebGL2.
//! Placed boxes are solid blocks in their clue's colour that tween in and out. JavaScript
//! owns the canvas events and forwards them, and calls `tick` + `render` on animation
//! frames only while something moves.

use std::sync::Arc;

use patches_core::{Board, BoxRegion, CELLS, Cell, Puzzle, cell_at};
use three_d::*;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::anim::Anim;
use crate::geom::{DOT_R, MARK_R, Pose, block_pose, cell_center, grid_lines, lattice_dots};
use crate::input::{Input, Moved, Released, Target};

const CAMERA_DISTANCE: f32 = 24.0;
const ORBIT_SPEED: f32 = 0.006;
const PITCH_RANGE: (f32, f32) = (0.15, 1.45);

/// Same colours as `PALETTE` in web/src/lib/puzzle.ts so the 3D view matches the layer grids.
const PALETTE: [Srgba; 16] = [
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
    Srgba::new_opaque(0xb1, 0xcd, 0xf1),
    Srgba::new_opaque(0xf1, 0xd8, 0xb1),
    Srgba::new_opaque(0xf1, 0xb1, 0xc2),
    Srgba::new_opaque(0xb1, 0xf1, 0xb1),
];

struct Block {
    region: BoxRegion,
    color: Srgba,
    anim: Anim,
    removing: bool,
}

impl Block {
    /// Grows out of its own centre.
    fn spawn(region: BoxRegion, color: Srgba) -> Self {
        let to = block_pose(&region);
        let from = Pose {
            center: to.center,
            half: to.half.map(|h| h * 0.2),
        };
        Self {
            region,
            color,
            anim: Anim::new(from, to),
            removing: false,
        }
    }

    /// Shrinks away; `tick` drops the block once it has vanished.
    fn dismiss(&mut self) {
        self.removing = true;
        self.anim.retarget(Pose {
            half: [0.0; 3],
            ..self.anim.pose()
        });
    }
}

#[wasm_bindgen]
pub struct Game {
    canvas: HtmlCanvasElement,
    context: Context,
    camera: Camera,
    yaw: f32,
    pitch: f32,
    /// One full-size cube per cell, used only for picking and never rendered.
    cells: InstancedMesh,
    wires: Gm<InstancedMesh, ColorMaterial>,
    /// White lattice dots plus a marker per clue. A block hides its own clue's marker.
    dots: Gm<InstancedMesh, ColorMaterial>,
    floor: Gm<Mesh, PhysicalMaterial>,
    blocks_mesh: Gm<InstancedMesh, PhysicalMaterial>,
    blocks: Vec<Block>,
    preview: Gm<Mesh, ColorMaterial>,
    preview_on: bool,
    ambient: AmbientLight,
    sun: DirectionalLight,
    puzzle: Puzzle,
    board: Board,
    input: Input,
    interactive: bool,
}

#[wasm_bindgen]
impl Game {
    /// `answer = true` shows the stored solution and ignores input.
    #[wasm_bindgen(constructor)]
    pub fn new(
        canvas: HtmlCanvasElement,
        puzzle_json: &str,
        answer: bool,
    ) -> Result<Game, JsValue> {
        console_error_panic_hook::set_once();
        let puzzle: Puzzle = serde_json::from_str(puzzle_json).map_err(err)?;
        let context = gl_context(&canvas)?;

        let camera = Camera::new_perspective(
            Viewport::new_at_origo(canvas.width(), canvas.height()),
            vec3(0.0, 0.0, CAMERA_DISTANCE),
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            Deg(35.0),
            0.1,
            100.0,
        );
        let cube = CpuMesh::cube();
        let cell_poses: Vec<Pose> = (0..CELLS)
            .map(|i| Pose {
                center: cell_center(cell_at(i)),
                half: [0.5; 3],
            })
            .collect();
        let lines = grid_lines();
        let mut game = Game {
            cells: instanced(&context, &cube, &cell_poses, vec![Srgba::WHITE; CELLS]),
            wires: Gm::new(
                instanced(&context, &cube, &lines, vec![Srgba::WHITE; lines.len()]),
                unlit(&context),
            ),
            dots: Gm::new(dots_mesh(&context, &puzzle), unlit(&context)),
            floor: floor_mesh(&context),
            blocks_mesh: Gm::new(
                instanced(&context, &cube, &[], Vec::new()),
                block_material(&context),
            ),
            blocks: Vec::new(),
            preview: preview_mesh(&context),
            preview_on: false,
            ambient: AmbientLight::new(&context, 0.6, Srgba::WHITE),
            sun: DirectionalLight::new(&context, 1.2, Srgba::WHITE, vec3(-0.5, -1.0, -0.7)),
            board: Board::new(puzzle.clues.clone()),
            canvas,
            camera,
            yaw: 0.7,
            pitch: 0.6,
            context,
            puzzle,
            input: Input::default(),
            interactive: !answer,
        };
        game.update_camera();
        if answer {
            for b in game.puzzle.solution.clone() {
                if !game.place(b) {
                    return Err("stored solution breaks the rules".into());
                }
            }
        }
        Ok(game)
    }

    pub fn render(&mut self) {
        let (w, h) = (self.canvas.width(), self.canvas.height());
        self.camera.set_viewport(Viewport::new_at_origo(w, h));
        let mut objects: Vec<&dyn Object> =
            vec![&self.floor, &self.wires, &self.dots, &self.blocks_mesh];
        if self.preview_on {
            objects.push(&self.preview);
        }
        RenderTarget::screen(&self.context, w, h)
            .clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 0.0, 1.0))
            .render(&self.camera, objects, &[&self.ambient, &self.sun]);
    }

    /// Advances the block tweens by `dt_ms`. Returns true while anything is still moving.
    pub fn tick(&mut self, dt_ms: f32) -> bool {
        let mut moving = false;
        for b in &mut self.blocks {
            moving |= b.anim.tick(dt_ms);
        }
        self.blocks.retain(|b| !(b.removing && b.anim.done()));
        self.sync_blocks();
        moving
    }

    /// Screen position (CSS px, top-left origin) of every clue's top face, flattened as `[x0, y0, x1, y1, ...]`.
    pub fn labels(&self) -> Vec<f32> {
        let s = self.scale();
        let h = self.canvas.height() as f32;
        self.puzzle
            .clues
            .iter()
            .flat_map(|c| {
                let top = Vec3::from(cell_center(c.cell)) + vec3(0.0, 0.5, 0.0);
                let p = self.camera.pixel_at_position(top);
                [p.x / s, (h - p.y) / s]
            })
            .collect()
    }

    /// Returns true when the board changed: a press on a block removes it.
    pub fn pointer_down(&mut self, x: f32, y: f32) -> bool {
        let target = self.target(x, y);
        if let Target::Block(c) = target {
            self.remove(c);
        }
        self.input.down((x, y), target);
        matches!(target, Target::Block(_))
    }

    /// Returns true when the scene needs a redraw.
    pub fn pointer_move(&mut self, x: f32, y: f32) -> bool {
        // The GPU pick only runs while a build is being dragged.
        let hover = if self.input.building() {
            self.pick_cell(x, y)
        } else {
            None
        };
        match self.input.moved((x, y), hover) {
            Moved::Orbit { dx, dy } => {
                self.yaw -= dx * ORBIT_SPEED;
                self.pitch = (self.pitch + dy * ORBIT_SPEED).clamp(PITCH_RANGE.0, PITCH_RANGE.1);
                self.update_camera();
                true
            }
            Moved::Preview(r) => {
                self.set_preview(Some(r));
                true
            }
            Moved::Nothing => false,
        }
    }

    /// Returns true when the board changed.
    pub fn pointer_up(&mut self, x: f32, y: f32) -> bool {
        let hover = if self.input.building() {
            self.pick_cell(x, y)
        } else {
            None
        };
        // A box that breaks the rules is simply not built.
        let changed = match self.input.up(hover) {
            Released::Place(r) => self.place(r),
            Released::Nothing => false,
        };
        // The first corner of a click-click box shows as a one-cell preview.
        self.set_preview(self.input.pending().map(|c| BoxRegion::spanning(c, c)));
        changed
    }

    pub fn pointer_cancel(&mut self) {
        self.input.cancel();
        self.set_preview(None);
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
            self.input.cancel();
            self.set_preview(None);
            for b in self.blocks.iter_mut().filter(|b| !b.removing) {
                b.dismiss();
            }
        }
    }
}

impl Game {
    fn place(&mut self, r: BoxRegion) -> bool {
        if self.board.place(r).is_err() {
            return false;
        }
        let clue = self.board.clue_of(self.board.boxes().len() - 1);
        self.blocks
            .push(Block::spawn(r, PALETTE[clue % PALETTE.len()]));
        true
    }

    fn remove(&mut self, cell: Cell) {
        self.board.remove_at(cell);
        if let Some(b) = self
            .blocks
            .iter_mut()
            .find(|b| !b.removing && b.region.contains(cell))
        {
            b.dismiss();
        }
    }

    fn sync_blocks(&mut self) {
        self.blocks_mesh.set_instances(&Instances {
            transformations: self
                .blocks
                .iter()
                .map(|b| transform(&b.anim.pose()))
                .collect(),
            colors: Some(self.blocks.iter().map(|b| b.color).collect()),
            ..Default::default()
        });
        // With nothing to cast, generate_shadow_map keeps the old map, so clear it first.
        self.sun.clear_shadow_map();
        self.sun
            .generate_shadow_map(1024, [&self.blocks_mesh])
            .expect("shadow map");
    }

    fn set_preview(&mut self, r: Option<BoxRegion>) {
        self.preview_on = r.is_some();
        if let Some(r) = r {
            self.preview.set_transformation(transform(&block_pose(&r)));
        }
    }

    fn target(&self, x: f32, y: f32) -> Target {
        if !self.interactive {
            return Target::Nothing;
        }
        match self.pick_cell(x, y) {
            None => Target::Nothing,
            Some(c) if self.board.box_at(c).is_some() => Target::Block(c),
            Some(c) => Target::Empty(c),
        }
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
}

// three-d wants an Arc'd context; WebGL is single-threaded, so Send/Sync never matters here.
#[allow(clippy::arc_with_non_send_sync)]
fn gl_context(canvas: &HtmlCanvasElement) -> Result<Context, JsValue> {
    let gl = canvas
        .get_context("webgl2")?
        .ok_or("WebGL2 is not available")?
        .dyn_into::<web_sys::WebGl2RenderingContext>()?;
    Context::from_gl_context(Arc::new(three_d::context::Context::from_webgl2_context(gl)))
        .map_err(err)
}

/// `CpuMesh::cube()` spans -1..1, so a pose's half-extents are its scale.
fn transform(p: &Pose) -> Mat4 {
    Mat4::from_translation(Vec3::from(p.center))
        * Mat4::from_nonuniform_scale(p.half[0], p.half[1], p.half[2])
}

fn instanced(
    context: &Context,
    shape: &CpuMesh,
    poses: &[Pose],
    colors: Vec<Srgba>,
) -> InstancedMesh {
    InstancedMesh::new(
        context,
        &Instances {
            transformations: poses.iter().map(transform).collect(),
            colors: Some(colors),
            ..Default::default()
        },
        shape,
    )
}

/// Flat colour, multiplied by each instance's colour.
fn unlit(context: &Context) -> ColorMaterial {
    ColorMaterial::new_opaque(
        context,
        &CpuMaterial {
            albedo: Srgba::WHITE,
            ..Default::default()
        },
    )
}

fn dots_mesh(context: &Context, puzzle: &Puzzle) -> InstancedMesh {
    let mut poses: Vec<Pose> = lattice_dots()
        .into_iter()
        .map(|center| Pose {
            center,
            half: [DOT_R; 3],
        })
        .collect();
    let mut colors = vec![Srgba::WHITE; poses.len()];
    for (i, clue) in puzzle.clues.iter().enumerate() {
        poses.push(Pose {
            center: cell_center(clue.cell),
            half: [MARK_R; 3],
        });
        colors.push(PALETTE[i % PALETTE.len()]);
    }
    instanced(context, &CpuMesh::sphere(12), &poses, colors)
}

/// Dark ground just below the bottom layer; it receives the blocks' shadows.
fn floor_mesh(context: &Context) -> Gm<Mesh, PhysicalMaterial> {
    let mut floor = Gm::new(
        Mesh::new(context, &CpuMesh::square()),
        PhysicalMaterial::new_opaque(
            context,
            &CpuMaterial {
                albedo: Srgba::new_opaque(0x27, 0x27, 0x2a),
                roughness: 1.0,
                metallic: 0.0,
                ..Default::default()
            },
        ),
    );
    floor.set_transformation(
        Mat4::from_translation(vec3(0.0, -4.3, 0.0))
            * Mat4::from_angle_x(Deg(-90.0))
            * Mat4::from_scale(5.0),
    );
    floor
}

/// White, lit, and tinted per instance by the clue colour.
fn block_material(context: &Context) -> PhysicalMaterial {
    PhysicalMaterial::new_opaque(
        context,
        &CpuMaterial {
            albedo: Srgba::WHITE,
            roughness: 0.8,
            metallic: 0.0,
            ..Default::default()
        },
    )
}

/// The translucent box under a drag, or the pending first corner of a click-click box.
fn preview_mesh(context: &Context) -> Gm<Mesh, ColorMaterial> {
    Gm::new(
        Mesh::new(context, &CpuMesh::cube()),
        ColorMaterial::new_transparent(
            context,
            &CpuMaterial {
                albedo: Srgba::new(255, 255, 255, 90),
                ..Default::default()
            },
        ),
    )
}

fn err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}
