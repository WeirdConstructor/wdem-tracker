use wctr_signal_ops::*;
use crate::gui_painter::GUIPainter;

#[derive(Debug, PartialEq, Clone)]
pub enum OperatorInputMode {
    Value(f32),
    RegInfo(String),
}

#[derive(Debug)]
pub struct OperatorInputSettings {
        simcom:       SimulatorCommunicator,
    pub specs:        Vec<(OpIOSpec, OpInfo)>,
    pub groups:       Vec<(String, Vec<usize>)>,
                    //     x/y/w/h,  op_i,  in_idx
        active_zones: Vec<([f32; 4], usize, usize)>,
        highlight: Option<(usize, usize)>,
        selection: Option<(usize, usize)>,
        orig_val:  f32,
    pub scroll_offs: (usize, usize),
}

fn draw_op<P>(p: &mut P, op: &(OpIOSpec, OpInfo), highlight: &Option<(usize, usize)>, selection: &Option<(usize, usize)>) -> (f32, f32, Vec<([f32; 4], usize, usize)>)
        where P: GUIPainter {
    let inp_col_w : f32 = 180.0;
    let inp_col_wr: f32 =  70.0;
    let padding   : f32 =   2.0;
    let text_h    : f32 =  12.0;

    let mut io_lens = op.0.input_values.len();
    if op.0.output_regs.len() > io_lens {
        io_lens = op.0.output_regs.len();
    }

    let op_h = (1 + io_lens) as f32 * text_h + padding * 2.0;
    let op_w = padding + inp_col_w + padding + inp_col_wr;

    p.draw_rect(
        [0.2, 0.2, 0.2, 1.0], [0.0, 0.0], [op_w, op_h], true, 0.1);
    p.draw_rect(
        [1.0, 0.0, 1.0, 1.0], [0.0, 0.0], [op_w, op_h], false, 0.5);

    p.add_offs(padding, padding);

    p.draw_text(
        [0.3, 1.0, 0.8, 1.0], [0.0, 0.0], text_h, format!("{}", op.1.name));
    p.draw_text(
        [1.0, 0.3, 0.3, 1.0], [inp_col_w - (text_h + padding), 0.0], text_h, "IN".to_string());
    p.draw_text(
        [0.3, 1.0, 0.3, 1.0], [inp_col_w + padding, 0.0], text_h, "OUT".to_string());

    let mut y = text_h;

    let mut active_zones : Vec<([f32; 4], usize, usize)> = Vec::new();

    for (idx, (i, is)) in op.0.input_values.iter().zip(op.0.inputs.iter()).enumerate() {
        let text = match i {
            OpIn::Constant(v) => {
                format!("{:>8.3}", *v)
            },
            OpIn::Reg(u) =>
                format!("r{}", *u),
            OpIn::RegMix2(u, u2, f) =>
                format!("r{}x{:0.2}[{:0.2}]", *u, *u2, *f),
            OpIn::RegAdd(u, f) =>
                format!("r{}+[{:0.2}]", *u, *f),
            OpIn::RegMul(u, f) =>
                format!("r{}*[{:0.2}]", *u, *f),
            OpIn::RegAddMul(u, f, f2) =>
                format!("(r{}+[{:0.2}])*[{:0.2}]", *u, *f, *f2),
            OpIn::RegMulAdd(u, f, f2) =>
                format!("(r{}*[{:0.2}])+[{:0.2}]", *u, *f, *f2),
            OpIn::RegLerp(u, f, f2) =>
                format!("r{}/[{:0.2}][{:0.2}]", *u, *f, *f2),
            OpIn::RegSStep(u, f, f2) =>
                format!("r{}~[{:0.2}][{:0.2}]", *u, *f, *f2),
            OpIn::RegMap(u, f, f2, g, g2) =>
                format!("r{}[{:0.2}-{:0.2}]->[{:0.2}-{:0.2}]", *u, *f, *f2, *g, *g2),
        };

        let o = p.get_offs();
        active_zones.push(([o.0, o.1 + y, inp_col_w, text_h], op.0.index, idx));

        let mut highlighted = if let Some((op_idx, i_idx)) = highlight {
            *op_idx == op.0.index && idx == *i_idx
        } else {
            false
        };

        // XXX: Because the mouse cursor is repositioned, we would
        //      get flickering on neighbour elements.
        if selection.is_some() {
            highlighted = false;
        }

        let selected = if let Some((op_idx, i_idx)) = selection {
            *op_idx == op.0.index && idx == *i_idx
        } else {
            false
        };

        p.draw_rect(
            if selected { [1.0, 0.2, 0.2, 1.0] }
            else        { [0.4, 0.4, 0.4, 1.0] },
            [0.0, y],
            [inp_col_w - 1.0, text_h - 1.0],
            !selected && highlighted,
            0.5);

        p.draw_text(
            [1.0, 0.3, 0.8, 1.0], [0.0, y], text_h,
            format!("{:<7} {}", is.name, text));

        y += text_h;
    }

    y = text_h;
    for (o, os) in op.0.output_regs.iter().zip(op.0.outputs.iter()) {
        p.draw_text(
            [1.0, 0.3, 0.8, 1.0], [inp_col_w + padding, y], text_h,
            format!("{:<7} r{}", os.name, o));
        y += text_h;
    }

    p.add_offs(0.0, -padding);
    p.draw_lines(
        [1.0, 0.0, 1.0, 1.0],
        [inp_col_w, 0.0],
        &vec![[0.0, 0.0], [0.0, op_h]],
        false,
        0.5);

    (op_w, op_h, active_zones)
}

impl OperatorInputSettings {
    pub fn new(simcom: SimulatorCommunicator) -> Self {
        OperatorInputSettings {
            simcom:        simcom,
            specs:         Vec::new(),
            groups:        Vec::new(),
            active_zones:  Vec::new(),
            highlight:     None,
            selection:     None,
            orig_val:      0.0,
            scroll_offs:   (0, 0),
        }
    }

    pub fn save_input_values(&mut self) -> Vec<(String, Vec<(String, OpIn)>)> {
        self.simcom.save_input_values()
    }

    pub fn load_input_values(&mut self, inputs: &Vec<(String, Vec<(String, OpIn)>)>) {
        self.simcom.load_input_values(inputs);
    }

    pub fn hit_zone(&mut self, x: f32, y: f32) -> Option<(usize, usize)> {
        for az in self.active_zones.iter() {
            if    x >= (az.0)[0]
               && y >= (az.0)[1]
               && x <= ((az.0)[0] + (az.0)[2])
               && y <= ((az.0)[1] + (az.0)[3]) {

               return Some((az.1, az.2));
            }
        }

        None
    }

    pub fn handle_mouse_move(&mut self, x: f32, y: f32, xr: f32, yr: f32, button_is_down: bool) -> bool {
        if !button_is_down { self.selection = None; }

        let old_highlight = self.highlight;

        self.highlight = None;

        for az in self.active_zones.iter() {
            if    x >= (az.0)[0]
               && y >= (az.0)[1]
               && x <= ((az.0)[0] + (az.0)[2])
               && y <= ((az.0)[1] + (az.0)[3]) {

                self.highlight = Some((az.1, az.2));

                if    button_is_down
                   && self.selection.is_none()
                   && old_highlight == self.highlight {

                    self.selection = self.highlight;
                    self.orig_val  = self.get_selection_val();
                    return true;
                }
                break;
            }
        }

        if self.selection.is_some() && button_is_down {
            let exp = (10.0 as f64).powf((xr / 200.0).abs() as f64);
            let ampli = -((yr as f64 * exp) / 200.0) as f32;
            let s = self.selection.unwrap();
            self.set_input_val(s.0, s.1, self.orig_val + ampli);
            return true;
        }

        false
    }

    pub fn set_input_default(&mut self, op_idx: usize, i_idx: usize) {
        let iname = self.specs[op_idx].0.inputs[i_idx].name.clone();
        let default = self.specs[op_idx].0.input_defaults[i_idx];
        self.specs[op_idx].0.input_values[i_idx] = default;
        self.simcom.set_op_input(op_idx, &iname, default, false);
    }

    pub fn set_input_val(&mut self, op_idx: usize, i_idx: usize, val: f32) {
        let iname = self.specs[op_idx].0.inputs[i_idx].name.clone();
        self.specs[op_idx].0.input_values[i_idx] = OpIn::Constant(val);
        self.simcom.set_op_input(op_idx, &iname, OpIn::Constant(val), false);
    }

    pub fn get_selection_val(&self) -> f32 {
        if self.selection.is_some() {
            let s = self.selection.unwrap();
            self.get_input_val(s.0, s.1)
        } else {
            0.0
        }
    }

    pub fn get_input_val(&self, op_idx: usize, i_idx: usize) -> f32 {
        if let OpIn::Constant(v) = self.specs[op_idx].0.input_values[i_idx] {
            v
        } else {
            0.0
        }
    }

    pub fn update(&mut self) {
        let r = self.simcom.update(|ev| {
            if let SimulatorUIEvent::OpSpecUpdate(up) = ev {
                Some(up)
            } else { None }
        });

        if r.is_some() {
            self.update_from_spec(r.unwrap().unwrap());
        }
    }

    fn update_from_spec(&mut self, specs: Vec<(OpIOSpec, OpInfo)>) {
        //d// println!("Updated: {:?}", specs);
        self.specs = specs;
        self.groups = Vec::new();

        for iv in self.specs.iter() {
            let group = &iv.1.group;
            if group.index <= self.groups.len() {
                self.groups.resize(group.index + 1, ("".to_string(), Vec::new()));
            }
        }

        for i in 0..self.groups.len() {
            let ops : Vec<usize> =
                self.specs
                    .iter()
                    .filter(|o| o.1.group.index == i)
                    .map(|o| o.0.index)
                    .collect();

            if ops.is_empty() { continue; }

            let group = self.specs[ops[0]].1.group.clone();
            println!("OP: {:?} => {}", ops, group.name);

            self.groups[i] = (group.name.clone(), ops);
        }
    }

    pub fn draw<P>(&mut self, p: &mut P) where P: GUIPainter {
        let text_h = 10.0;

        self.active_zones = Vec::new();

        let oo = p.get_offs();

        let mut skip_groups_count = self.scroll_offs.1;
        for grp in self.groups.iter() {
            if skip_groups_count > 0 {
                skip_groups_count -= 1;
                continue;
            }

            p.draw_text([1.0, 1.0, 1.0, 1.0], [0.0, 0.0], text_h, grp.0.clone());

            let ooo = p.get_offs();

            let mut skip_ops_count = self.scroll_offs.0;

            let mut max_op_h = 0.0;
            for op_i in grp.1.iter() {
                if skip_ops_count > 0 {
                    skip_ops_count -= 1;
                    continue;
                }

                let op = &self.specs[*op_i];
                let o = p.get_offs();
                p.set_offs((o.0, o.1 + text_h));

                let (w, h, zones) = draw_op(p, op, &self.highlight, &self.selection);
                self.active_zones.extend_from_slice(&zones);
                if h > max_op_h { max_op_h = h; }
                p.set_offs((o.0 + w + 3.0, o.1));

                if (p.get_offs().0 - oo.0) > p.get_area_size().0 {
                    break;
                }

            }

            p.set_offs(ooo);

            p.add_offs(0.0, max_op_h + text_h);
        }

        p.set_offs(oo);
    }
}

