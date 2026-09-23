//! The 3D board: a white wireframe of the 4x4x4 cube, rendered with three-d on WebGL2.
//! Each clue is a marker in its box's shape. Placed boxes are solid blocks in their clue's
//! colour, or red outlines when they break a rule, and tween in and out. JavaScript owns
//! the canvas events and forwards them, and calls `tick` + `render` on animation frames
//! only while something moves.

use std::sync::Arc;

use patches_core::{Board, BoxRegion, Cell, Puzzle};
use three_d::*;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::anim::Anim;
use crate::geom::{
    DOT_R, MARK_R, Pose, WHOLE, block_pose, box_edges, cell_center, clip, grid_lines, lattice_dots,
    marker_half, peeled,
};
use crate::input::{Input, Moved, Released, Target};
use crate::pick::pick;

/// Close enough to fill the view, far enough that the whole cube fits from every angle.
const CAMERA_DISTANCE: f32 = 12.0;
/// How much closer the camera gets at full zoom.
const ZOOM_RANGE: f32 = 4.0;
/// Zoomed in at least this far (0..1), the layer nearest the camera is peeled away.
const PEEL_AT: f32 = 0.5;
/// On solving, blocks move this much further from the centre, so the cube opens up.
const SPREAD: f32 = 1.12;
/// Opacity of the grid lines: faint hairlines you look through.
const WIRE_ALPHA: u8 = 56;
const ORBIT_SPEED: f32 = 0.006;
/// Just short of straight up and straight down, so the camera can go all the way around.
const PITCH_RANGE: (f32, f32) = (-1.5, 1.5);
/// Outline of a box that breaks a rule, like Patches' red patch.
const WRONG: Srgba = Srgba::new_opaque(0xef, 0x44, 0x44);

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
    /// Breaks a rule: drawn as a red outline instead of a solid block.
    wrong: bool,
    anim: Anim,
    removing: bool,
}

impl Block {
    /// Grows out of its own centre.
    fn spawn(region: BoxRegion, color: Srgba, wrong: bool) -> Self {
        let to = block_pose(&region);
        let from = Pose {
            center: to.center,
            half: to.half.map(|h| h * 0.2),
        };
        Self {
            region,
            color,
            wrong,
            anim: Anim::new(from, to),
            removing: false,
        }
    }

    /// Brightens and eases outward: the solved cube opening up.
    fn celebrate(&mut self) {
        self.color = vivid(self.color);
        let p = block_pose(&self.region);
        self.anim.retarget(Pose {
            center: p.center.map(|c| c * SPREAD),
            ..p
        });
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
    /// 0 (whole cube in view) to 1 (closest, nearest layer peeled).
    zoom: f32,
    /// The cells drawn and pickable: the whole cube, or the cube without a peeled layer.
    shown: BoxRegion,
    wires: Gm<InstancedMesh, ColorMaterial>,
    /// White lattice dots plus a round marker per any-shape clue.
    dots: Gm<InstancedMesh, ColorMaterial>,
    /// A small cuboid in the clue's shape per shaped clue. A block hides its own clue's marker.
    markers: Gm<InstancedMesh, PhysicalMaterial>,
    blocks_mesh: Gm<InstancedMesh, PhysicalMaterial>,
    outlines: Gm<InstancedMesh, ColorMaterial>,
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
    /// False on the answer page.
    playable: bool,
    /// Set when the player solves the puzzle; the board then locks until Reset.
    solved: bool,
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
        let lines = grid_lines(&WHOLE);
        let (dot_poses, dot_colors) = dot_instances(&puzzle, &WHOLE);
        let (marker_poses, marker_colors) = marker_instances(&puzzle, &WHOLE);
        let mut game = Game {
            wires: Gm::new(
                instanced(&context, &cube, &lines, vec![Srgba::WHITE; lines.len()]),
                see_through(&context, WIRE_ALPHA),
            ),
            dots: Gm::new(
                instanced(&context, &CpuMesh::sphere(12), &dot_poses, dot_colors),
                unlit(&context),
            ),
            markers: Gm::new(
                instanced(&context, &cube, &marker_poses, marker_colors),
                block_material(&context),
            ),
            blocks_mesh: Gm::new(
                instanced(&context, &cube, &[], Vec::new()),
                block_material(&context),
            ),
            outlines: Gm::new(instanced(&context, &cube, &[], Vec::new()), unlit(&context)),
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
            zoom: 0.0,
            shown: WHOLE,
            context,
            puzzle,
            input: Input::default(),
            playable: !answer,
            solved: false,
        };
        game.update_camera();
        if answer {
            for b in game.puzzle.solution.clone() {
                if !game.place(b) || game.board.fault(&b).is_some() {
                    return Err("stored solution breaks the rules".into());
                }
            }
        }
        Ok(game)
    }

    pub fn render(&mut self) {
        let (w, h) = (self.canvas.width(), self.canvas.height());
        self.camera.set_viewport(Viewport::new_at_origo(w, h));
        let mut objects: Vec<&dyn Object> = vec![
            &self.wires,
            &self.dots,
            &self.markers,
            &self.blocks_mesh,
            &self.outlines,
        ];
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

    /// Screen position (CSS px, top-left origin) of every clue's top face, flattened as
    /// `[x0, y0, x1, y1, ...]`; NaN for a clue in a peeled layer.
    pub fn labels(&self) -> Vec<f32> {
        let s = self.scale();
        let h = self.canvas.height() as f32;
        self.puzzle
            .clues
            .iter()
            .flat_map(|c| {
                if !self.shown.contains(c.cell) {
                    return [f32::NAN; 2];
                }
                let top = Vec3::from(cell_center(c.cell)) + vec3(0.0, 0.5, 0.0);
                let p = self.camera.pixel_at_position(top);
                [p.x / s, (h - p.y) / s]
            })
            .collect()
    }

    /// Returns true when the press landed on a cell.
    pub fn pointer_down(&mut self, x: f32, y: f32) -> bool {
        let target = self.target(x, y);
        self.input.down((x, y), target);
        self.set_hover(None);
        target != Target::Nothing
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
        // A box that would overlap another is not built; one that breaks a rule is built
        // and shows as a red outline.
        let changed = match self.input.up(self.pick_cell(x, y)) {
            Released::Place(r) => self.place(r),
            Released::Remove(r) => self.remove(r.min),
            Released::Replace { old, new } => self.extend(old, new),
            Released::Nothing => false,
        };
        if changed && self.playable && self.board.is_solved() {
            self.celebrate();
        }
        // The first corner of a click-click box shows as a one-cell preview.
        self.set_preview(self.input.pending().map(|c| BoxRegion::spanning(c, c)));
        self.set_hover(self.pick_cell(x, y));
        changed
    }

    /// Zooms by `delta` (the full range is 1). Returns false at either end, so the page
    /// can scroll instead.
    pub fn zoom_by(&mut self, delta: f32) -> bool {
        let zoom = (self.zoom + delta).clamp(0.0, 1.0);
        if zoom == self.zoom {
            return false;
        }
        self.zoom = zoom;
        self.update_camera();
        true
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

    /// Placed boxes that keep every rule.
    pub fn box_count(&self) -> u32 {
        self.blocks
            .iter()
            .filter(|b| !b.removing && !b.wrong)
            .count() as u32
    }

    /// Placed boxes that break a rule.
    pub fn wrong_count(&self) -> u32 {
        self.blocks
            .iter()
            .filter(|b| !b.removing && b.wrong)
            .count() as u32
    }

    pub fn is_solved(&self) -> bool {
        self.board.is_solved()
    }

    pub fn reset(&mut self) {
        if self.playable {
            self.solved = false;
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
    fn celebrate(&mut self) {
        self.solved = true;
        self.input.cancel();
        for b in self.blocks.iter_mut().filter(|b| !b.removing) {
            b.celebrate();
        }
    }

    fn place(&mut self, r: BoxRegion) -> bool {
        if self.board.place(r).is_err() {
            return false;
        }
        let (color, wrong) = self.look(&r);
        self.blocks.push(Block::spawn(r, color, wrong));
        true
    }

    /// The clue's colour, and whether the box breaks a rule.
    fn look(&self, r: &BoxRegion) -> (Srgba, bool) {
        let color = self
            .board
            .clue_of(r)
            .map_or(WRONG, |c| PALETTE[c % PALETTE.len()]);
        (color, self.board.fault(r).is_some())
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

    /// Grows box `old` into `new` unless that would overlap another box; the block tweens to
    /// its new size.
    fn extend(&mut self, old: BoxRegion, new: BoxRegion) -> bool {
        if new == old || self.board.replace(old, new).is_err() {
            return false;
        }
        let (color, wrong) = self.look(&new);
        if let Some(b) = self
            .blocks
            .iter_mut()
            .find(|b| !b.removing && b.region == old)
        {
            b.region = new;
            (b.color, b.wrong) = (color, wrong);
            b.anim.retarget(block_pose(&new));
        }
        true
    }

    fn sync_blocks(&mut self) {
        let (wrong, solid): (Vec<&Block>, Vec<&Block>) = self.blocks.iter().partition(|b| b.wrong);
        let (mut poses, mut colors) = (Vec::new(), Vec::new());
        for b in solid {
            if let Some(p) = clip(&b.anim.pose(), &self.shown) {
                poses.push(p);
                colors.push(b.color);
            }
        }
        self.blocks_mesh.set_instances(&instances(&poses, colors));
        let edges: Vec<Pose> = wrong
            .iter()
            .filter_map(|b| clip(&b.anim.pose(), &self.shown))
            .flat_map(|p| box_edges(&p))
            .collect();
        self.outlines
            .set_instances(&instances(&edges, vec![WRONG; edges.len()]));
        // With nothing to cast, generate_shadow_map keeps the old map, so clear it first.
        self.sun.clear_shadow_map();
        self.sun
            .generate_shadow_map(1024, [&self.blocks_mesh])
            .expect("shadow map");
    }

    fn set_preview(&mut self, r: Option<BoxRegion>) {
        // A hair bigger than the block, so it never z-fights the block it extends.
        let pose = r.and_then(|r| {
            let p = block_pose(&r);
            let padded = Pose {
                half: p.half.map(|h| h + 0.02),
                ..p
            };
            clip(&padded, &self.shown)
        });
        self.preview_on = pose.is_some();
        if let Some(p) = pose {
            self.preview.set_transformation(transform(&p));
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

    /// The cell under the pointer (CSS px); none on the answer page or once solved.
    fn pick_cell(&self, x: f32, y: f32) -> Option<Cell> {
        if !self.playable || self.solved {
            return None;
        }
        let (origin, dir) = self.ray(x, y);
        pick(origin, dir, self.board.boxes(), &self.shown)
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
        let eye = vec3(cp * sy, sp, cp * cy) * (CAMERA_DISTANCE - self.zoom * ZOOM_RANGE);
        self.camera
            .set_view(eye, vec3(0.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0));
        let shown = if self.zoom >= PEEL_AT {
            peeled(eye.into())
        } else {
            WHOLE
        };
        if shown != self.shown {
            self.shown = shown;
            self.show_scenery();
            self.sync_blocks();
        }
    }

    /// Redraws the lattice and clue markers for the shown cells.
    fn show_scenery(&mut self) {
        let lines = grid_lines(&self.shown);
        self.wires
            .set_instances(&instances(&lines, vec![Srgba::WHITE; lines.len()]));
        let (poses, colors) = dot_instances(&self.puzzle, &self.shown);
        self.dots.set_instances(&instances(&poses, colors));
        let (poses, colors) = marker_instances(&self.puzzle, &self.shown);
        self.markers.set_instances(&instances(&poses, colors));
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
    InstancedMesh::new(context, &instances(poses, colors), shape)
}

fn instances(poses: &[Pose], colors: Vec<Srgba>) -> Instances {
    Instances {
        transformations: poses.iter().map(transform).collect(),
        colors: Some(colors),
        ..Default::default()
    }
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

/// White lattice dots, plus a round marker for each shown clue that allows any shape.
fn dot_instances(puzzle: &Puzzle, shown: &BoxRegion) -> (Vec<Pose>, Vec<Srgba>) {
    let mut poses: Vec<Pose> = lattice_dots(shown)
        .into_iter()
        .map(|center| Pose {
            center,
            half: [DOT_R; 3],
        })
        .collect();
    let mut colors = vec![Srgba::WHITE; poses.len()];
    for (i, clue) in puzzle.clues.iter().enumerate() {
        if clue.shape.is_none() && shown.contains(clue.cell) {
            poses.push(Pose {
                center: cell_center(clue.cell),
                half: [MARK_R; 3],
            });
            colors.push(PALETTE[i % PALETTE.len()]);
        }
    }
    (poses, colors)
}

/// A small cuboid in its box's shape for each shown clue that names one.
fn marker_instances(puzzle: &Puzzle, shown: &BoxRegion) -> (Vec<Pose>, Vec<Srgba>) {
    let (mut poses, mut colors) = (Vec::new(), Vec::new());
    for (i, clue) in puzzle.clues.iter().enumerate() {
        if let Some(shape) = clue.shape
            && shown.contains(clue.cell)
        {
            poses.push(Pose {
                center: cell_center(clue.cell),
                half: marker_half(shape),
            });
            colors.push(PALETTE[i % PALETTE.len()]);
        }
    }
    (poses, colors)
}

/// A saturated version of a pastel from `PALETTE`: 0xb1 channels go deep, 0xf1 stay bright.
fn vivid(c: Srgba) -> Srgba {
    let f = |v: u8| ((f32::from(v) - 177.0) / 64.0 * 207.0 + 48.0).clamp(0.0, 255.0) as u8;
    Srgba::new_opaque(f(c.r), f(c.g), f(c.b))
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
