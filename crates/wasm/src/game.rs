//! The 3D board: a white wireframe of the 4x4x4 cube, rendered with three-d on WebGL2.
//! Each clue is a marker in its box's shape. Placed boxes are solid blocks in their clue's
//! colour that tween in and out. JavaScript
//! owns the canvas events and forwards them, and calls `tick` + `render` on animation
//! frames only while something moves.

use std::sync::Arc;

use patches_core::{Board, BoxRegion, Cell, Puzzle};
use three_d::*;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::anim::Anim;
use crate::geom::{
    DOT_R, MARK_R, Pose, block_pose, cell_center, grid_lines, lattice_dots, marker_half,
};
use crate::input::{Input, Moved, Released, Target};
use crate::pick::pick;

/// Close enough to fill the view, far enough that the whole cube fits from every angle.
const CAMERA_DISTANCE: f32 = 12.0;
/// Opacity of the grid lines: faint hairlines you look through.
const WIRE_ALPHA: u8 = 56;
const ORBIT_SPEED: f32 = 0.006;
/// Just short of straight up and straight down, so the camera can go all the way around.
const PITCH_RANGE: (f32, f32) = (-1.5, 1.5);

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
    wires: Gm<InstancedMesh, ColorMaterial>,
    /// White lattice dots plus a round marker per any-shape clue.
    dots: Gm<InstancedMesh, ColorMaterial>,
    /// A small cuboid in the clue's shape per shaped clue. A block hides its own clue's marker.
    markers: Gm<InstancedMesh, PhysicalMaterial>,
    blocks_mesh: Gm<InstancedMesh, PhysicalMaterial>,
    blocks: Vec<Block>,
    preview: Gm<Mesh, ColorMaterial>,
    preview_on: bool,
    /// The cell a click would pick, while no button is down.
    hover: Option<Cell>,
    hover_mesh: Gm<Mesh, ColorMaterial>,
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
        let lines = grid_lines();
        let mut game = Game {
            wires: Gm::new(
                instanced(&context, &cube, &lines, vec![Srgba::WHITE; lines.len()]),
                see_through(&context, WIRE_ALPHA),
            ),
            dots: Gm::new(dots_mesh(&context, &puzzle), unlit(&context)),
            markers: markers_mesh(&context, &puzzle),
            blocks_mesh: Gm::new(
                instanced(&context, &cube, &[], Vec::new()),
                block_material(&context),
            ),
            blocks: Vec::new(),
            preview: translucent(&context, 90),
            preview_on: false,
            hover: None,
            hover_mesh: translucent(&context, 40),
            ambient: AmbientLight::new(&context, 0.6, Srgba::WHITE),
            sun: DirectionalLight::new(&context, 1.2, Srgba::WHITE, vec3(-0.5, -1.0, -0.7)),
            board: Board::new(puzzle.clues.clone()),
            canvas,
            camera,
            // Off the cube's diagonal, so no two cell centres line up on screen.
            yaw: 0.65,
            pitch: 0.4,
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
            vec![&self.wires, &self.dots, &self.markers, &self.blocks_mesh];
        if self.preview_on {
            objects.push(&self.preview);
        }
        if self.hover.is_some() {
            objects.push(&self.hover_mesh);
        }
        RenderTarget::screen(&self.context, w, h)
            // Opaque black: faint lines blend against it, not against a see-through canvas.
            .clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0))
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

    pub fn pointer_down(&mut self, x: f32, y: f32) {
        let target = self.target(x, y);
        self.input.down((x, y), target);
        self.set_hover(None);
    }

    /// `t` is the event's timestamp in ms. Returns true when the scene needs a redraw.
    pub fn pointer_move(&mut self, x: f32, y: f32, t: f64) -> bool {
        if !self.input.pressed() {
            return self.set_hover(self.pick_cell(x, y));
        }
        // Where the pointer is only matters while a build is being dragged.
        let hover = if self.input.building() {
            self.pick_cell(x, y)
        } else {
            None
        };
        match self.input.moved((x, y), hover, t) {
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
        // A box that breaks the rules is simply not built.
        let changed = match self.input.up(self.pick_cell(x, y)) {
            Released::Place(r) => self.place(r),
            Released::Remove(r) => self.remove(r.min),
            Released::Replace { old, new } => self.extend(old, new),
            Released::Nothing => false,
        };
        // The first corner of a click-click box shows as a one-cell preview.
        self.set_preview(self.input.pending().map(|c| BoxRegion::spanning(c, c)));
        self.set_hover(self.pick_cell(x, y));
        changed
    }

    pub fn pointer_cancel(&mut self) {
        self.input.cancel();
        self.set_preview(None);
        self.set_hover(None);
    }

    /// Returns true when the scene needs a redraw.
    pub fn pointer_leave(&mut self) -> bool {
        self.set_hover(None)
    }

    /// Whether the pointer is over a cell a click would pick.
    pub fn hovering(&self) -> bool {
        self.hover.is_some()
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

    /// Returns true when a box was removed.
    fn remove(&mut self, cell: Cell) -> bool {
        if let Some(b) = self
            .blocks
            .iter_mut()
            .find(|b| !b.removing && b.region.contains(cell))
        {
            b.dismiss();
        }
        self.board.remove_at(cell)
    }

    /// Grows box `old` into `new` if the rules allow it; the block tweens to its new size.
    fn extend(&mut self, old: BoxRegion, new: BoxRegion) -> bool {
        if new == old || self.board.replace(old, new).is_err() {
            return false;
        }
        if let Some(b) = self
            .blocks
            .iter_mut()
            .find(|b| !b.removing && b.region == old)
        {
            b.region = new;
            b.anim.retarget(block_pose(&new));
        }
        true
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
            // A hair bigger than the block, so it never z-fights the block it extends.
            let p = block_pose(&r);
            self.preview.set_transformation(transform(&Pose {
                half: p.half.map(|h| h + 0.02),
                ..p
            }));
        }
    }

    /// Returns true when the highlighted cell changed.
    fn set_hover(&mut self, cell: Option<Cell>) -> bool {
        if cell == self.hover {
            return false;
        }
        self.hover = cell;
        if let Some(c) = cell {
            self.hover_mesh.set_transformation(transform(&Pose {
                center: cell_center(c),
                half: [0.5; 3],
            }));
        }
        true
    }

    fn target(&self, x: f32, y: f32) -> Target {
        match self.pick_cell(x, y) {
            None => Target::Nothing,
            Some(c) => match self.board.box_at(c) {
                Some(i) => Target::Block {
                    cell: c,
                    region: self.board.boxes()[i],
                },
                None => Target::Empty(c),
            },
        }
    }

    /// The cell under the pointer (CSS px); none on the answer page.
    fn pick_cell(&self, x: f32, y: f32) -> Option<Cell> {
        if !self.interactive {
            return None;
        }
        let (origin, dir) = self.ray(x, y);
        pick(origin, dir, self.board.boxes())
    }

    /// The camera ray through a point on the canvas (CSS px).
    fn ray(&self, x: f32, y: f32) -> ([f32; 3], [f32; 3]) {
        let s = self.scale();
        let pixel = (x * s, self.canvas.height() as f32 - y * s);
        (
            self.camera.position_at_pixel(pixel).into(),
            self.camera.view_direction_at_pixel(pixel).into(),
        )
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

/// White lattice dots, plus a round marker for each clue that allows any shape.
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
        if clue.shape.is_none() {
            poses.push(Pose {
                center: cell_center(clue.cell),
                half: [MARK_R; 3],
            });
            colors.push(PALETTE[i % PALETTE.len()]);
        }
    }
    instanced(context, &CpuMesh::sphere(12), &poses, colors)
}

/// A small lit cuboid in its box's shape for each clue that names one.
fn markers_mesh(context: &Context, puzzle: &Puzzle) -> Gm<InstancedMesh, PhysicalMaterial> {
    let (mut poses, mut colors) = (Vec::new(), Vec::new());
    for (i, clue) in puzzle.clues.iter().enumerate() {
        if let Some(shape) = clue.shape {
            poses.push(Pose {
                center: cell_center(clue.cell),
                half: marker_half(shape),
            });
            colors.push(PALETTE[i % PALETTE.len()]);
        }
    }
    Gm::new(
        instanced(context, &CpuMesh::cube(), &poses, colors),
        block_material(context),
    )
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

/// A see-through white box: the preview under a drag, the pending first corner of a
/// click-click box, or the hovered cell.
fn translucent(context: &Context, alpha: u8) -> Gm<Mesh, ColorMaterial> {
    Gm::new(
        Mesh::new(context, &CpuMesh::cube()),
        see_through(context, alpha),
    )
}

/// Flat white at opacity `alpha`, multiplied by each instance's colour.
fn see_through(context: &Context, alpha: u8) -> ColorMaterial {
    ColorMaterial::new_transparent(
        context,
        &CpuMaterial {
            albedo: Srgba::new(255, 255, 255, alpha),
            ..Default::default()
        },
    )
}

fn err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}
