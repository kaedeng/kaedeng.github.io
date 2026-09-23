//! The 3D board: a white wireframe of the 4x4x4 cube, rendered with three-d on WebGL2.
//! Each clue is a dashed marker in its box's shape, or a jack when any shape will do.
//! Placed boxes are solid blocks in their clue's colour with darker edges, or red outlines
//! when they break a rule, and tween in and out. JavaScript owns the canvas events and
//! forwards them, and calls `tick` + `render` on animation frames only while something
//! moves.

use std::f32::consts::FRAC_PI_4;
use std::sync::Arc;

use patches_core::{Board, BoxRegion, Cell, N, Puzzle};
use three_d::*;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::anim::Anim;
use crate::geom::{
    DASH_R, DOT_R, EDGE_R, JACK_R, OUTLINE_R, Pose, WHOLE, block_pose, box_edges, by_depth,
    cell_center, clip, corners, cut_faces, dashed_edges, grid_lines, jack, marker_half,
    marker_weight, nearness, peeled, region_edges,
};
use crate::input::{Input, Moved, Released, Target};
use crate::keys::{Cmd, Cursor, Dir, Keys, Press, axis_for};
use crate::pick::{hidden, pick};
use crate::view::{Flat, facing, orbit};

/// Close enough to fill the view, far enough that the whole cube fits from every angle
/// (its corners are 3.46 from the centre; sin 25° × 8.5 is 3.6).
const CAMERA_DISTANCE: f32 = 8.5;
/// Wide enough that the back of the cube looks clearly smaller than the front.
const FIELD_OF_VIEW: Deg<f32> = Deg(50.0);
/// CSS px kept clear around the layer in 2D, so the view cube in the corner never covers
/// a cell.
const FLAT_MARGIN: f32 = 80.0;
/// How much closer the camera gets at full zoom.
const ZOOM_RANGE: f32 = 2.8;
/// Zoomed in at least this far (0..1), the layer nearest the camera is peeled away.
const PEEL_AT: f32 = 0.5;
/// On solving, blocks move this much further from the centre, so the cube opens up.
const SPREAD: f32 = 1.12;
/// Opacity of the inner grid lines, nearest to farthest: faint hairlines you look through,
/// fading with depth so the near side reads first.
const WIRE_ALPHA: (f32, f32) = (0.3, 0.07);
/// Opacity of the shown cells' 12 outer edges and 8 corner dots, nearest to farthest.
const FRAME_ALPHA: (f32, f32) = (0.75, 0.2);
const ORBIT_SPEED: f32 = 0.006;
/// Just short of straight up and straight down, so the camera can go all the way around.
const PITCH_RANGE: (f32, f32) = (-1.5, 1.5);
/// Outline of a box that breaks a rule, like Patches' red patch.
const WRONG: Srgba = Srgba::new_opaque(0xef, 0x44, 0x44);
/// Brightness of a block's edges and section hatching against its colour.
const DARKER: f32 = 0.6;
/// How far up the screen a clue's label sits from its cell's centre: still inside the
/// cell, and in 2D clear of its marker.
const LABEL_LIFT: f32 = 0.34;

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

    /// Eases outward: the solved cube opening up.
    fn celebrate(&mut self) {
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
    /// Set while one layer is shown face-on; `yaw`, `pitch` and `zoom` then wait for the
    /// return to 3D.
    flat: Option<Flat>,
    /// The cells drawn and pickable: the whole cube, the cube without a peeled layer, or the
    /// one layer shown in 2D.
    shown: BoxRegion,
    /// The shown cells' grid lines, fading with depth; their 12 outer edges bolder.
    wires: Gm<InstancedMesh, ColorMaterial>,
    /// A white dot on each of the shown cells' 8 corners, fading with depth like the lines.
    dots: Gm<InstancedMesh, ColorMaterial>,
    /// Per clue, a dashed outline in its box's shape, or a jack for any shape. A block
    /// hides its own clue's marker.
    markers: Gm<InstancedMesh, ColorMaterial>,
    blocks_mesh: Gm<InstancedMesh, PhysicalMaterial>,
    /// Each solid block's edges in a darker shade, so blocks of like colours stay apart.
    edges: Gm<InstancedMesh, ColorMaterial>,
    /// Diagonal stripes over each block's cut face while a layer is peeled in 3D.
    hatch: Gm<InstancedMesh, Stripes>,
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
    /// Clue `i` and its box are drawn in `colors[i]`.
    colors: Vec<Srgba>,
    board: Board,
    input: Input,
    keys: Keys,
    /// The keyboard's cell, drawn as a white outline once the keyboard is in use.
    cursor: Cursor,
    cursor_on: bool,
    cursor_mesh: Gm<InstancedMesh, ColorMaterial>,
    /// False on the answer page.
    playable: bool,
    /// Set when the player solves the puzzle; the board then locks until Reset.
    solved: bool,
}

#[wasm_bindgen]
impl Game {
    /// `answer = true` shows the stored solution and ignores input. `colors` holds each
    /// clue's colour as `0xRRGGBB`, in clue order.
    #[wasm_bindgen(constructor)]
    pub fn new(
        canvas: HtmlCanvasElement,
        puzzle_json: &str,
        answer: bool,
        colors: &[u32],
    ) -> Result<Game, JsValue> {
        console_error_panic_hook::set_once();
        let puzzle: Puzzle = serde_json::from_str(puzzle_json).map_err(err)?;
        let colors: Vec<Srgba> = colors
            .iter()
            .map(|&c| Srgba::new_opaque((c >> 16) as u8, (c >> 8) as u8, c as u8))
            .collect();
        let context = gl_context(&canvas)?;

        let camera = Camera::new_perspective(
            Viewport::new_at_origo(canvas.width(), canvas.height()),
            vec3(0.0, 0.0, CAMERA_DISTANCE),
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            FIELD_OF_VIEW,
            0.1,
            100.0,
        );
        let cube = CpuMesh::cube();
        // The lattice and markers change with the camera, so `update_camera` below fills
        // them in.
        let mut game = Game {
            wires: Gm::new(
                instanced(&context, &cube, &[], Vec::new()),
                see_through(&context, 255),
            ),
            dots: Gm::new(
                instanced(&context, &CpuMesh::sphere(12), &[], Vec::new()),
                see_through(&context, 255),
            ),
            markers: Gm::new(instanced(&context, &cube, &[], Vec::new()), unlit(&context)),
            blocks_mesh: Gm::new(
                instanced(&context, &cube, &[], Vec::new()),
                block_material(&context),
            ),
            edges: Gm::new(instanced(&context, &cube, &[], Vec::new()), unlit(&context)),
            hatch: Gm::new(instanced(&context, &cube, &[], Vec::new()), Stripes),
            outlines: Gm::new(instanced(&context, &cube, &[], Vec::new()), unlit(&context)),
            cursor_mesh: Gm::new(instanced(&context, &cube, &[], Vec::new()), unlit(&context)),
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
            flat: None,
            shown: WHOLE,
            context,
            puzzle,
            colors,
            input: Input::default(),
            keys: Keys::default(),
            cursor: Cursor::new([N - 1; 3]),
            cursor_on: false,
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

    /// Takes up the canvas's drawing buffer size, after JavaScript changed it: the camera
    /// gets the new shape, so the cube isn't stretched and labels stay on their cells.
    pub fn resize(&mut self) {
        let (w, h) = (self.canvas.width(), self.canvas.height());
        self.camera.set_viewport(Viewport::new_at_origo(w, h));
        self.update_camera();
    }

    pub fn render(&mut self) {
        let (w, h) = (self.canvas.width(), self.canvas.height());
        self.camera.set_viewport(Viewport::new_at_origo(w, h));
        let mut objects: Vec<&dyn Object> = vec![
            &self.wires,
            &self.dots,
            &self.markers,
            &self.blocks_mesh,
            &self.edges,
            &self.hatch,
            &self.outlines,
            &self.cursor_mesh,
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

    /// Screen position (CSS px, top-left origin) of a point inside every clue's cell, up
    /// the screen from its centre so the label clears most of the marker, flattened as
    /// `[x0, y0, x1, y1, ...]`; NaN for a clue in a hidden layer.
    pub fn labels(&self) -> Vec<f32> {
        let (yaw, pitch) = self.angles();
        let lift = Vec3::from(orbit(yaw, pitch).1) * LABEL_LIFT;
        self.puzzle
            .clues
            .iter()
            .flat_map(|c| {
                if !self.shown.contains(c.cell) {
                    return [f32::NAN; 2];
                }
                self.css_point(Vec3::from(cell_center(c.cell)) + lift)
            })
            .collect()
    }

    /// Screen position (CSS px, top-left origin) of `[x, y, z]`'s centre as `[x, y]`, e.g.
    /// to aim a demo pointer at it; NaN while its layer is hidden.
    pub fn cell_point(&self, x: u8, y: u8, z: u8) -> Vec<f32> {
        let cell = [x, y, z];
        if !self.shown.contains(cell) {
            return vec![f32::NAN; 2];
        }
        self.css_point(Vec3::from(cell_center(cell))).to_vec()
    }

    /// The `[axis, sign]` of the cube face turned most toward the camera, as `view_face`
    /// takes them; in 2D, the face shown.
    pub fn facing(&self) -> Vec<i32> {
        let (yaw, pitch) = self.angles();
        let (axis, sign) = facing(yaw, pitch);
        vec![axis as i32, i32::from(sign)]
    }

    /// The placed boxes, six numbers each: `min`, then `max`.
    pub fn boxes(&self) -> Vec<u8> {
        self.board
            .boxes()
            .iter()
            .flat_map(|b| b.min.into_iter().chain(b.max))
            .collect()
    }

    /// Puts the placed boxes back to `boxes` (as `boxes` gives them), e.g. after a demo:
    /// the others shrink away and the missing ones grow back. A solve this undoes is
    /// undone too, so the board takes input again.
    pub fn set_boxes(&mut self, boxes: &[u8]) {
        let want: Vec<BoxRegion> = boxes
            .as_chunks::<6>()
            .0
            .iter()
            .map(|b| BoxRegion {
                min: [b[0], b[1], b[2]],
                max: [b[3], b[4], b[5]],
            })
            .collect();
        let extra: Vec<BoxRegion> = self
            .board
            .boxes()
            .iter()
            .filter(|b| !want.contains(b))
            .copied()
            .collect();
        for b in extra {
            self.remove(b.min);
        }
        for b in want {
            if !self.board.boxes().contains(&b) {
                self.place(b);
            }
        }
        if self.solved && !self.board.is_solved() {
            self.solved = false;
            for b in self.blocks.iter_mut().filter(|b| !b.removing) {
                b.anim.retarget(block_pose(&b.region));
            }
        }
    }

    /// Per clue, 1 when a solid block other than its own stands between the camera and its
    /// cell, so its label can hide; else 0.
    pub fn hidden_labels(&self) -> Vec<u8> {
        let eye: [f32; 3] = self.camera.position().into();
        let solid: Vec<BoxRegion> = self
            .blocks
            .iter()
            .filter(|b| !b.wrong && !b.removing)
            .map(|b| b.region)
            .collect();
        self.puzzle
            .clues
            .iter()
            .map(|c| u8::from(hidden(eye, c.cell, &solid, &self.shown)))
            .collect()
    }

    /// Returns true when the press landed on a cell.
    pub fn pointer_down(&mut self, x: f32, y: f32) -> bool {
        let target = self.target(x, y);
        self.input.down((x, y), target);
        self.set_hover(None);
        // The keyboard carries on from where the mouse pressed. A box half drawn stays, so
        // this click can finish it.
        if let Some(c) = self.pick_cell(x, y) {
            self.cursor.cell = c;
            self.show_cursor();
            self.show_selection();
        }
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
                self.leave_flat();
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
        let released = match self.input.up(self.pick_cell(x, y)) {
            Released::Tap(c) => self.cursor.tap(c),
            Released::Cancel => {
                self.cursor.cancel();
                Released::Nothing
            }
            // A box the mouse built, grew or removed ends any box half drawn; a turn doesn't.
            other => {
                if other != Released::Nothing {
                    self.cursor.cancel();
                }
                other
            }
        };
        let changed = self.apply(released);
        self.show_selection();
        self.set_hover(self.pick_cell(x, y));
        changed
    }

    /// Handles a key press (`KeyboardEvent.key`). Returns true when it was a board key, so
    /// the page should not act on it too.
    pub fn key(&mut self, key: &str, shift: bool) -> bool {
        match self.keys.press(key, shift, self.cursor.selecting()) {
            Press::Ignored => false,
            Press::Consumed => true,
            Press::Run(cmd) => {
                self.run(cmd);
                true
            }
        }
    }

    /// Shows or hides the keyboard cursor, e.g. when the board gains or loses focus.
    pub fn set_cursor_visible(&mut self, on: bool) {
        self.cursor_on = on && self.playable && !self.solved;
        self.show_cursor();
    }

    /// A vim-style status line, empty unless vim keys are on or a `:` command is typed.
    pub fn mode_line(&self) -> String {
        self.keys.mode_line(self.cursor.selecting())
    }

    /// Zooms by `delta` (the full range is 1). Returns false at either end and in 2D, so
    /// the page can scroll instead.
    pub fn zoom_by(&mut self, delta: f32) -> bool {
        let zoom = (self.zoom + delta).clamp(0.0, 1.0);
        if zoom == self.zoom || self.flat.is_some() {
            return false;
        }
        self.zoom = zoom;
        self.update_camera();
        true
    }

    /// Shows one layer face-on from outside a face (`axis` 0-2 is x, y, z; `sign` is 1 for
    /// the + side), starting with the face's own layer. On the face already shown, goes back
    /// to the 3D view it came from.
    pub fn view_face(&mut self, axis: usize, sign: i8) {
        let same = self.flat.is_some_and(|f| f.axis == axis && f.sign == sign);
        self.set_flat((!same).then(|| Flat::new(axis, sign)));
    }

    /// In 2D, shows the layer `d` layers deeper (d > 0) or nearer the face (d < 0).
    pub fn step_layer(&mut self, d: i8) {
        if let Some(mut f) = self.flat {
            f.step(-f.sign * d);
            self.set_flat(Some(f));
        }
    }

    /// How many layers in from its face the 2D view is (0 is the face's own layer); -1 in 3D.
    pub fn view_depth(&self) -> i32 {
        self.flat.map_or(-1, |f| i32::from(f.depth()))
    }

    /// How far the camera is from the cube's centre, in cells, e.g. to give a view cube the
    /// same perspective.
    pub fn view_distance(&self) -> f32 {
        self.camera.position().magnitude()
    }

    /// The camera's `[yaw, pitch]` in radians, e.g. to turn a view cube with it.
    pub fn view_angles(&self) -> Vec<f32> {
        let (yaw, pitch) = self.angles();
        vec![yaw, pitch]
    }

    /// Drops the press, e.g. when a second finger turns it into a pinch. A box half drawn
    /// stays, as after a turn.
    pub fn pointer_cancel(&mut self) {
        self.input.cancel();
        self.show_selection();
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
            self.cursor.cancel();
            self.set_preview(None);
            self.show_cursor();
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
        self.cursor.cancel();
        self.cursor_on = false;
        self.show_cursor();
        for b in self.blocks.iter_mut().filter(|b| !b.removing) {
            b.celebrate();
        }
    }

    /// Carries out a finished press or key command. A box that would overlap another is not
    /// built; one that breaks a rule is built and shows as a red outline. Returns true when
    /// the board changed.
    fn apply(&mut self, released: Released) -> bool {
        let changed = match released {
            Released::Place(r) => self.place(r),
            Released::Remove(r) => self.remove(r.min),
            Released::Replace { old, new } => self.extend(old, new),
            Released::Nothing | Released::Tap(_) | Released::Cancel => false,
        };
        if changed && self.playable && self.board.is_solved() {
            self.celebrate();
        }
        changed
    }

    /// Turning and zooming work anywhere; the rest only on a board still in play.
    fn run(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Turn(d) => {
                self.leave_flat();
                self.yaw += f32::from(d) * FRAC_PI_4;
                self.update_camera();
            }
            Cmd::Zoom(d) => {
                self.zoom_by(f32::from(d) * 0.25);
            }
            _ if self.playable && !self.solved => self.run_on_board(cmd),
            _ => {}
        }
    }

    fn run_on_board(&mut self, cmd: Cmd) {
        self.cursor_on = true;
        match cmd {
            Cmd::Move(dir, n) => self.move_cursor(dir, n as i8),
            Cmd::Select => {
                let block = self
                    .board
                    .box_at(self.cursor.cell)
                    .map(|i| self.board.boxes()[i]);
                let released = self.cursor.select(block);
                self.apply(released);
            }
            Cmd::Remove => {
                self.cursor.cancel();
                self.remove(self.cursor.cell);
            }
            Cmd::Cancel => self.cursor.cancel(),
            Cmd::Clue(d) => self.jump_to_clue(d),
            Cmd::Top => self.cursor.cell[1] = self.shown.max[1],
            Cmd::Bottom => self.cursor.cell[1] = self.shown.min[1],
            Cmd::Turn(_) | Cmd::Zoom(_) => {}
        }
        self.show_cursor();
        self.show_selection();
    }

    /// Moves the cursor `n` cells. In 2D, a move into or out of the screen turns to the
    /// next layer and takes the cursor along.
    fn move_cursor(&mut self, dir: Dir, n: i8) {
        let (axis, sign) = match self.flat {
            Some(f) => f.axis_for(dir),
            None => axis_for(dir, self.yaw),
        };
        if let Some(mut f) = self.flat.filter(|f| f.axis == axis) {
            f.step(sign * n);
            self.set_flat(Some(f));
        }
        self.cursor.step(axis, sign * n, &self.shown);
    }

    /// Switches between 2D (`Some`) and 3D.
    fn set_flat(&mut self, flat: Option<Flat>) {
        self.flat = flat;
        self.update_camera();
        self.show_selection();
    }

    /// Goes back to 3D, turned to look where the 2D view did, e.g. when the player orbits.
    fn leave_flat(&mut self) {
        if let Some(f) = self.flat {
            let (yaw, pitch) = f.angles();
            self.yaw = yaw;
            self.pitch = pitch.clamp(PITCH_RANGE.0, PITCH_RANGE.1);
            self.set_flat(None);
        }
    }

    /// Moves the cursor to the next (`d` = 1) or previous (-1) shown clue.
    fn jump_to_clue(&mut self, d: i8) {
        let clues: Vec<Cell> = self
            .puzzle
            .clues
            .iter()
            .map(|c| c.cell)
            .filter(|&c| self.shown.contains(c))
            .collect();
        let n = clues.len() as i32;
        if n == 0 {
            return;
        }
        let next = match clues.iter().position(|&c| c == self.cursor.cell) {
            Some(i) => (i as i32 + i32::from(d)).rem_euclid(n),
            None if d > 0 => 0,
            None => n - 1,
        };
        self.cursor.cell = clues[next as usize];
    }

    fn show_cursor(&mut self) {
        let edges = if self.cursor_on {
            box_edges(
                &Pose {
                    center: cell_center(self.cursor.cell),
                    half: [0.5; 3],
                },
                OUTLINE_R,
            )
        } else {
            Vec::new()
        };
        self.cursor_mesh
            .set_instances(&instances(&edges, vec![Srgba::WHITE; edges.len()]));
    }

    /// The box being drawn, begun by a click or the keyboard. In 2D it is cut to the shown
    /// layer, and the cursor follows the layer, so a first corner on another layer shows
    /// where it lines up on this one.
    fn show_selection(&mut self) {
        self.set_preview(self.cursor.selection());
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
        let color = self.board.clue_of(r).map_or(WRONG, |c| self.colors[c]);
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
        // Only a peel in 3D cuts a section; in 2D every face is a cut.
        let hatching = self.flat.is_none();
        let (mut solid, mut edges, mut cuts, mut wrong) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for b in &self.blocks {
            let pose = b.anim.pose();
            let Some(shown) = clip(&pose, &self.shown) else {
                continue;
            };
            if b.wrong {
                wrong.extend(box_edges(&shown, OUTLINE_R).into_iter().map(|e| (e, WRONG)));
                continue;
            }
            let dark = darker(b.color);
            solid.push((shown, b.color));
            edges.extend(box_edges(&shown, EDGE_R).into_iter().map(|e| (e, dark)));
            if hatching {
                cuts.extend(cut_faces(&pose, &self.shown).into_iter().map(|f| (f, dark)));
            }
        }
        self.blocks_mesh.set_instances(&paired(solid));
        self.edges.set_instances(&paired(edges));
        self.hatch.set_instances(&paired(cuts));
        self.outlines.set_instances(&paired(wrong));
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
                Some(i) => Target::Block(self.board.boxes()[i]),
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

    /// Where a world point shows on the canvas, in CSS px from its top-left.
    fn css_point(&self, p: Vec3) -> [f32; 2] {
        let s = self.scale();
        let h = self.canvas.height() as f32;
        let p = self.camera.pixel_at_position(p);
        [p.x / s, (h - p.y) / s]
    }

    /// Physical pixels per CSS pixel of the canvas.
    fn scale(&self) -> f32 {
        self.canvas.width() as f32 / self.canvas.client_width().max(1) as f32
    }

    /// The orbit angles in use: the 2D view's face, else the 3D camera's.
    fn angles(&self) -> (f32, f32) {
        self.flat.map_or((self.yaw, self.pitch), |f| f.angles())
    }

    fn update_camera(&mut self) {
        let (yaw, pitch) = self.angles();
        let (out, up) = orbit(yaw, pitch);
        let distance = match self.flat {
            Some(_) => CAMERA_DISTANCE,
            None => CAMERA_DISTANCE - self.zoom * ZOOM_RANGE,
        };
        let eye = Vec3::from(out) * distance;
        self.camera
            .set_view(eye, vec3(0.0, 0.0, 0.0), Vec3::from(up));
        let shown = match self.flat {
            Some(f) => {
                let height = self.flat_height() / distance;
                self.camera.set_orthographic_projection(height, 0.1, 100.0);
                f.region()
            }
            None => {
                self.camera
                    .set_perspective_projection(FIELD_OF_VIEW, 0.1, 100.0);
                if self.zoom >= PEEL_AT {
                    peeled(eye.into())
                } else {
                    WHOLE
                }
            }
        };
        if shown != self.shown {
            self.shown = shown;
            self.cursor.clamp(&shown);
            self.sync_blocks();
            self.show_cursor();
        }
        self.show_markers();
        self.show_lattice();
    }

    /// World units from the bottom to the top of the canvas in 2D: the layer fills the
    /// smaller side less `FLAT_MARGIN` each way, or half of it on a tiny canvas.
    fn flat_height(&self) -> f32 {
        let (w, h) = (self.canvas.width() as f32, self.canvas.height() as f32);
        let side = w.min(h);
        let cell = (side - 2.0 * FLAT_MARGIN * self.scale()).max(side / 2.0) / f32::from(N);
        h / cell.max(1.0)
    }

    /// Redraws the clue markers for the shown cells and the camera: nearer ones in bolder
    /// lines.
    fn show_markers(&mut self) {
        let weights: Vec<f32> = self
            .puzzle
            .clues
            .iter()
            .map(|c| marker_weight(self.clue_nearness(c.cell)))
            .collect();
        let (poses, colors) = marker_instances(&self.puzzle, &self.colors, &weights, &self.shown);
        self.markers.set_instances(&instances(&poses, colors));
    }

    /// How near the camera clue cell `c` is (0..1), to weigh its marker's lines. In 2D the
    /// one layer shown is all at one depth, so every clue gets the plain weight.
    fn clue_nearness(&self, c: Cell) -> f32 {
        if self.flat.is_some() {
            return 0.5;
        }
        let eye: [f32; 3] = self.camera.position().into();
        nearness(cell_center(c), eye, &self.shown)
    }

    /// Redraws the lattice of the shown cells for the camera: each line and corner dot
    /// fades with its depth, and the 12 outer edges and corners stand out.
    fn show_lattice(&mut self) {
        let eye: [f32; 3] = self.camera.position().into();
        let fade = |p: [f32; 3], alpha| faded(nearness(p, eye, &self.shown), alpha);
        let inner = grid_lines(&self.shown).into_iter().map(|l| (l, WIRE_ALPHA));
        let outer = region_edges(&self.shown)
            .into_iter()
            .map(|e| (e, FRAME_ALPHA));
        let lines = inner.chain(outer).map(|(l, a)| (l, fade(l.center, a)));
        self.wires.set_instances(&paired(lines.collect()));
        let dots = corners(&self.shown).into_iter().map(|center| {
            let dot = Pose {
                center,
                half: [DOT_R; 3],
            };
            (dot, fade(center, FRAME_ALPHA))
        });
        self.dots.set_instances(&paired(dots.collect()));
    }
}

/// White at an opacity between the `(near, far)` ends of `alpha`, by `nearness` (0..1).
fn faded(nearness: f32, alpha: (f32, f32)) -> Srgba {
    let a = by_depth(nearness, alpha);
    Srgba::new(255, 255, 255, (a * 255.0).round() as u8)
}

/// `color` at `DARKER` brightness: a block's edges and section hatching.
fn darker(color: Srgba) -> Srgba {
    let dim = |c: u8| (f32::from(c) * DARKER).round() as u8;
    Srgba::new_opaque(dim(color.r), dim(color.g), dim(color.b))
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

/// `instances` from `(pose, colour)` pairs.
fn paired(items: Vec<(Pose, Srgba)>) -> Instances {
    let (poses, colors): (Vec<Pose>, Vec<Srgba>) = items.into_iter().unzip();
    instances(&poses, colors)
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

/// For each shown clue, in its colour and with its lines `weights[i]` times their plain
/// thickness: a dashed outline in its box's shape, filled with the page's black so it hides
/// what lies behind it, or a jack when any shape will do.
fn marker_instances(
    puzzle: &Puzzle,
    colors: &[Srgba],
    weights: &[f32],
    shown: &BoxRegion,
) -> (Vec<Pose>, Vec<Srgba>) {
    let (mut poses, mut out) = (Vec::new(), Vec::new());
    for ((clue, &color), &w) in puzzle.clues.iter().zip(colors).zip(weights) {
        if !shown.contains(clue.cell) {
            continue;
        }
        let center = cell_center(clue.cell);
        let marker = match clue.shape {
            Some(shape) => {
                let body = Pose {
                    center,
                    half: marker_half(shape),
                };
                poses.push(body);
                out.push(Srgba::BLACK);
                dashed_edges(&body, DASH_R * w)
            }
            None => jack(center, JACK_R * w),
        };
        out.extend(std::iter::repeat_n(color, marker.len()));
        poses.extend(marker);
    }
    (poses, out)
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

/// Diagonal stripes a fixed distance apart in world units, in each instance's colour. The
/// rest is discarded, so the block face under them shows through: a section's hatching.
struct Stripes;

/// On any face across an axis, x + y + z runs diagonally, so its level lines are stripes
/// at 45 degrees; 0.16 apart in the sum puts them about 0.11 apart on the face. Each
/// stripe takes 30% of that, so the hatching stays light.
const STRIPES_FRAG: &str = "
in vec3 pos;
in vec4 col;

layout (location = 0) out vec4 outColor;

void main()
{
    if (fract((pos.x + pos.y + pos.z) / 0.16) > 0.3) discard;
    outColor = vec4(color_mapping(col.rgb), 1.0);
}
";

impl Material for Stripes {
    fn fragment_shader_source(&self, _lights: &[&dyn Light]) -> String {
        format!("{}{}", ColorMapping::fragment_shader_source(), STRIPES_FRAG)
    }

    /// three-d leaves ids below 0x5000 to materials defined outside it.
    fn id(&self) -> EffectMaterialId {
        EffectMaterialId(0x0001)
    }

    fn use_uniforms(&self, program: &Program, viewer: &dyn Viewer, _lights: &[&dyn Light]) {
        viewer.color_mapping().use_uniforms(program);
    }

    fn render_states(&self) -> RenderStates {
        RenderStates::default()
    }

    fn material_type(&self) -> MaterialType {
        MaterialType::Opaque
    }
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
