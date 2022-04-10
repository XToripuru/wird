use std::path::*;
use std::fs::*;

fn main() {

    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 3 {
        if args[1] == "expand" {
            let path = PathBuf::from(&*args[2]);
            let code = read_to_string(&*args[2]).unwrap();
            let pkg = if args.len() >= 4 {
                &*args[3]
            } else {
                "wird"
            };

            codegen(code, path.file_name().unwrap().to_str().unwrap(), pkg);
        }
    }
    if args.len() == 2 {
        if args[1] == "host" {
            let _child = std::process::Command::new("cmd.exe")
            .arg("/C").arg("http-server exp").spawn().unwrap();
        }
    }
    
}

fn codegen(code: String, name: &str, pkg: &str) {
    let out = PathBuf::from("./exp");
    if !out.exists() {
        let src = PathBuf::from("./exp/src");
        create_dir_all(&src).unwrap();
        write("./exp/Cargo.toml", format!("[package]
name = \"{}\"
version = \"0.0.0\"
edition = \"2018\"

[lib]
crate-type = [\"cdylib\", \"rlib\"]

[dependencies]
wasm-bindgen = \"0.2\"", pkg)).unwrap();
    }

    std::fs::copy("./index.html", "./exp/index.html").unwrap();

    let mut javascript = String::new();
    let mut rust = String::new();
    let mut fns = vec![];

    for gen in generate(code) {
        match gen {
            Codegen::Javascript { code } => {
                javascript.push_str(&*code);
            }
            Codegen::Static { code } => {
                let tokens: Vec<&str> = code.split(" ").collect();
                for k in 0..tokens.len() {
                    if tokens[k].trim() == "#[wasm]" {
                        for p in (k+1).. {
                            if tokens[p] == "fn" && tokens[p-1] == "pub" {
                                let name = tokens[p+1].split("(").next().unwrap();
                                fns.push(name.into());
                                break;
                            }
                        }
                    }
                }
                rust.push_str(&*code);
            }
            Codegen::Expression { capture, expr, ret, n } => {
                fns.push(format!("V_A{}", n));
                if let Some(capture) = capture {
                    rust.push_str(
                        &*format!("#[wasm] pub fn V_A{}({}) -> {} {{ {} }}",
                        n,
                        capture.into_iter().map(|(n, t)| format!("{}: {}", n, t)).collect::<Vec<String>>().join(", "),
                        ret,
                        expr
                    ));
                } else {
                    rust.push_str(
                        &*format!("#[wasm] pub fn V_A{}() -> {} {{ {} }}",
                        n,
                        ret,
                        expr
                    ));
                }
                
            }
        }
    }

    write(format!("./exp/{}", name), format!("import init from './pkg/{}.js';
import {{ {} }} from './pkg/{}.js';

function run() {{
{}
}}

init().then(run);", pkg, fns.join(", "), pkg, javascript)).unwrap();

    write("./exp/src/lib.rs", format!("use wasm_bindgen::prelude::wasm_bindgen as wasm;\r\n{}", rust)).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(1000));
    
    let child = std::process::Command::new("cmd.exe")
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .arg("/C").arg("wasm-pack build exp --target web").spawn().unwrap();

    let output = child.wait_with_output().unwrap();
    let out = String::from_utf8(output.stderr).unwrap();
    let out: Vec<&str> = out.split("\n").collect();
    if out[out.len()-2].contains("ready") {
        println!("Built successfully!");
    } else {
        println!("Error occured during compilation");
        println!("{}", out.join("\n"));
    }
}

fn generate(mut code: String) -> Vec<Codegen> {

    let ch: Vec<char> = code.chars().collect();
    let s: Vec<bool> = {
        let mut r = Vec::with_capacity(ch.len());
        let mut init = None;
        for k in 0..ch.len() {
            if let Some(a) = init {
                if a == ch[k] {
                    for b in 0.. {
                        if ch[k-1-b] != '\\' {
                            if b%2 == 0 {
                                init = None;
                            }
                            break;
                        }
                    }
                }
            } else if (ch[k] == '"' || ch[k] == '\'' || ch[k] == '`') && (k == 0 || ch[k-1] != '\\') {
                init = Some(ch[k]);
            }
            r.push(init.is_some());
        }
        r
    };

    let mut gens = vec![];
    let mut nexpr = 0;
    let mut drains = vec![];
    
    let mut k = 0;
    while k < ch.len() {

        if !s[k] && ch[k] == '#' && (ch[k+1] == '{' || ch[k+1] == '[') {

            let mut capture = None;

            let mut n = k;

            // Check if capture is present and parse it
            if ch[k+1] == '[' {
                let mut b: i32 = 1;
                for mut p in (n+2).. {

                    b += if !s[p] && ch[p] == '[' {1}
                    else if !s[p] && ch[p] == ']' {-1}
                    else {0};

                    if b == 0 {
                        let cap = {
                            let mut cap: String = code[n..p+1].into();
                            cap = cap[2..(cap.len()-1)].into();
                            cap
                        };

                        let mut captures: Vec<(String, String)> = vec![];
                        
                        let mut last = 0;
                        let mut b = 0;
                        for (k, c) in cap.chars().enumerate() {
                            match c {
                                '(' => b += 1,
                                ')' => b -= 1,
                                ',' if b == 0 => {
                                    let mut tokens = cap[last..k].split(":");
                                    captures.push((tokens.next().unwrap().trim().into(), tokens.next().unwrap().trim().into()));
                                    last = k + 1;
                                }
                                _ => {}
                            }
                        }
                        let mut tokens = cap[last..].split(":");
                        captures.push((tokens.next().unwrap().trim().into(), tokens.next().unwrap().trim().into()));

                        capture = Some(captures);
                        n = p;
                        break;
                    }
                }
            }

            let mut b: i32 = 1;
            for mut p in (n+2).. {

                b += if !s[p] && ch[p] == '{' {1}
                else if !s[p] && ch[p] == '}' {-1}
                else {0};

                if b == 0 {
                    let mut block = {
                        let mut block: String = code[n..p+1].into();
                        block = block[2..(block.len()-1)].into();
                        block
                    };
                    
                    let mut ret = None;

                    if ch[p+1] == ';' {
                        p += 1;
                    } else {
                        for a in (p+1).. {
                            if !s[a] && ch[a] == ';' {
                                ret = Some({
                                    let mut ret: String = code[(p+1)..a].into();
                                    ret = ret.trim()[2..].trim().into();
                                    ret
                                });
                                p = a;
                                break;
                            }
                        }
                    }

                    if let Some(ret) = ret {
                        block = block.trim().into();
                        let name = format!("V_A{}", nexpr);
                        //println!("[expression] [{}] [{}] -> [{}]", name, block, ret);

                        gens.push(Codegen::Expression { capture: capture.clone(), expr: block, ret, n: nexpr });
                        if let Some(capture) = capture {
                            drains.push(
                                (k..(p+1),
                                Some(
                                    format!("{}({});",
                                    name,
                                    capture.into_iter().map(|(n, _)| n).collect::<Vec<String>>().join(", ")
                                )
                            )));
                        } else {
                            drains.push((k..(p+1), Some(format!("{}();", name))));
                        }
                        nexpr += 1;
                    } else {
                        //println!("[static] [{}]", block);
                        gens.push(Codegen::Static { code: block });
                        drains.push((k..(p+1), None));
                    }

                    k = p;
                    break;
                }
            }
        }

        k += 1;
    }

    let mut roff = 0;
    for (d, rep) in drains {
        let (a, b) = (d.start, d.end);
        if let Some(rep) = &rep {
            code.replace_range((a-roff)..(b-roff), rep);
        } else {
            code.drain((a-roff)..(b-roff));
        }
        roff += b - a;
    }

    //println!("[pure] {}", code);
    gens.push(Codegen::Javascript { code });

    gens
}

enum Codegen {
    Javascript {
        code: String
    },
    Static {
        code: String
    },
    Expression {
        capture: Option<Vec<(String, String)>>,
        expr: String,
        ret: String,
        n: usize,
    }
}










































// /*
// weather,      Normal, Sunny, Rainy
// food ration,
// resources,
// minerals??,
// humidity,
// temperature,
// ant health and performance,
// avg health and performance,
// send ants to work
// ant types,
// buildings, like walls to make higher temperature or vents to make lower temperature and something similar with humidity
// level of magazine
// special species will have special things to do
// */

// #![allow(unused)]
// use crossterm::{
//     ExecutableCommand, QueueableCommand,
//     terminal, event::{poll, read, Event, KeyCode, KeyEvent},
//     cursor::{self, *}, style::{self, *}, Result, execute, queue
// };
// use std::io::*;
// use std::time::{Instant, Duration};
// use std::sync::mpsc::{Sender, Receiver, channel};

// fn main() {

//     let mut scr = Screen::new();

//     let mut state = State::pickspec();
//     state.update(&mut scr, Update::Init);

//     //scr.text("Myrmica", scr.w/2, 5, Align::Left, Color::Red);
//     //scr.text("Myrmica", scr.w/2, 6, Align::Center, Color::Green);
//     //scr.text("Myrmica", scr.w/2, 7, Align::Right, Color::Blue);

//     loop {

//         state.update(&mut scr, Update::Render);

//         // g.update();

//         // scr.text(
//         //     format!("Day {}", g.time/(24*60*1000)),
//         //     scr.w - 1,
//         //     scr.h -1,
//         //     Align::Right,
//         //     Color::Grey
//         // );

//         if let Some(e) = scr.event() {
//             match e {
//                 Event::Key(k) => state.update(&mut scr, Update::KeyEvent(k)),
//                 Event::Resize(w, h) => {
//                     scr.w = w as usize;
//                     scr.h = h as usize;
//                     scr.buff = vec![(' ', Color::White, None); w as usize * h as usize];
//                     state.update(&mut scr, Update::Resize);
//                 }
//                 _ => {}
//             }
//         }
        
//         scr.render();
//     }

// }

// enum State {
//     PickSpec {
//         specs: Vec<Spec>,
//         n: usize,
//     },
//     PickArea {
//         spec: Spec,
//         areas: Vec<Area>,
//         n: usize
//     },
//     Game {
//         game: Game
//     }
// }

// #[derive(Clone, Copy, PartialEq, Eq)]
// enum Update {
//     Init,
//     KeyEvent(KeyEvent),
//     Resize,
//     Render
// }

// impl State {
//     fn update(&mut self, s: &mut Screen, up: Update) {
//         let mut new = None;
//         match self {
//             State::PickSpec { specs, n } if match up {
//                 Update::Resize | Update::Init | Update::KeyEvent(_) => true,
//                 _  => false
//             } => {
//                 match up {
//                     Update::KeyEvent(KeyEvent { code: KeyCode::Right, ..}) if *n < specs.len()-1 => *n += 1,
//                     Update::KeyEvent(KeyEvent { code: KeyCode::Left, ..}) if *n > 0 => *n -= 1,
//                     Update::KeyEvent(KeyEvent { code: KeyCode::Enter, ..}) => new =  Some(State::pickarea(specs[*n])),
//                     _ => {}
//                 };
//                 if *n < specs.len()-1 {
//                     s.text("==>", s.w/2 + s.w/4, s.h/2 - 1, Align::Right, Color::White, None);
//                 }
//                 if *n > 0 {
//                     s.text("<==", s.w/2 - s.w/4, s.h/2 - 1, Align::Left, Color::White, None);
//                 }
//                 s.text("press ENTER to select", s.w/2, s.h - 3, Align::Center, Color::Grey, None);
//                 specs[*n].render(s);
//             },
//             State::PickArea { spec, areas, n } if match up {
//                 Update::Resize | Update::Init | Update::KeyEvent(_) => true,
//                 _  => false
//             } => {
//                 match up {
//                     Update::KeyEvent(KeyEvent { code: KeyCode::Right, ..}) if *n < areas.len()-1 => *n += 1,
//                     Update::KeyEvent(KeyEvent { code: KeyCode::Left, ..}) if *n > 0 => *n -= 1,
//                     Update::KeyEvent(KeyEvent { code: KeyCode::Backspace, ..}) => new =  Some(State::pickspec()),
//                     //Update::KeyEvent(KeyEvent { code: KeyCode::Enter, ..}) => new =  Some(State::pickarea(specs[*n])),
//                     _ => {}
//                 };
//                 if *n < areas.len()-1 {
//                     s.text("==>", s.w/2 + s.w/4, s.h/2 - 1, Align::Right, Color::White, None);
//                 }
//                 if *n > 0 {
//                     s.text("<==", s.w/2 - s.w/4, s.h/2 - 1, Align::Left, Color::White, None);
//                 }
//                 s.text("press BACKSPACE to go back", 0, s.h - 1, Align::Left, Color::DarkGrey, None);
//                 s.text("press ENTER to select", s.w/2, s.h - 3, Align::Center, Color::Grey, None);
//                 areas[*n].render(s);
//             }
//             State::Game { game } if match up {
//                 Update::Resize | Update::Render | Update::Init | Update::KeyEvent(_) => true,
//                 _  => false
//             } => {
//                 //s.text("press BACKSPACE to go back", 0, s.h - 1, Align::Left, Color::DarkGrey, None);
//             }
//             _ => {}
//         };
//         if let Some(new) = new {
//             *self = new;
//             s.clear();
//             self.update(s, Update::Init);
//         }
//     }
//     fn pickspec() -> Self {
//         State::PickSpec {
//             specs: vec![
//                 Spec::FormicaFusca,
//                 Spec::MyrmicaRubra,
//                 Spec::MessorBarbarus
//             ],
//             n: 0
//         }
//     }
//     fn pickarea(spec: Spec) -> Self {
//         State::PickArea {
//             spec,
//             areas: vec![
//                 Area::MiddleterraneanForest,
//                 Area::MiddleterraneanAcre,
//             ],
//             n: 0
//         }
//     }
//     fn game(spec: Spec, area: Area) -> Self {
//         State::Game {
//             game: Game::new(spec, area)
//         }
//     }
// }

// #[derive(Clone, Copy, PartialEq, Eq)]
// enum Spec {
//     FormicaFusca,
//     MyrmicaRubra,
//     MessorBarbarus
// }

// impl Spec {
//     fn render(&self, s: &mut Screen) {
//         // 149, 125, 173
//         use Spec::*;
//         s.text(self.name(), s.w/2, s.h/2 - 2, Align::Center, Color::Rgb { r: 206, g: 58, b: 132 }, None);
//         let (queen, worker) = self.size();
//         s.text(format!("Queen {}-{}mm    Worker {}-{}mm", queen.0, queen.1, worker.0, worker.1), s.w/2, s.h/2, Align::Center, Color::Rgb { r: 224, g: 187, b: 228 }, None);
//         let (low, high) = self.temperature();
//         s.text(format!("Temperature {}~{}°C", low, high), s.w/2, s.h/2 + 1, Align::Center, Color::Rgb { r: 255, g: 179, b: 71 }, None);
//         s.text(format!("Humidity ~{}%", self.humidity()), s.w/2, s.h/2 + 2, Align::Center, Color::Rgb { r: 79, g: 185, b: 226 }, None);

//         match self {
//             FormicaFusca => {
//                 //s.text("- Increased search speed -", s.w/2, s.h/2 + 3, Align::Center, Color::Rgb { r: 119, g: 221, b: 119 }, None);
//             }
//             MyrmicaRubra => {
//                 s.text("- Can have multiple queens -", s.w/2, s.h/2 + 4, Align::Center, Color::Rgb { r: 119, g: 221, b: 119 }, None);
//             }
//             MessorBarbarus => {
//                 s.text("- Some worker ants become soliders -", s.w/2, s.h/2 + 4, Align::Center, Color::Rgb { r: 119, g: 221, b: 119 }, None);
//             }
//         }
//     }
//     fn name(&self) -> String {
//         use Spec::*;
//         match self {
//             FormicaFusca => "FORMICA FUSCA".into(),
//             MyrmicaRubra => "MYRMICA RUBRA".into(),
//             MessorBarbarus => "MESSOR BARBARUS".into(),
//         }
//     }
//     fn temperature(&self) -> (i32, i32) {
//         use Spec::*;
//         match self {
//             FormicaFusca => (20, 28),
//             MyrmicaRubra => (20, 28),
//             MessorBarbarus => (20, 28)
//         }
//     }
//     fn size(&self) -> ((i32, i32), (i32, i32)) {
//         use Spec::*;
//         match self {
//             FormicaFusca => ((10, 12), (3, 6)),
//             MyrmicaRubra => ((7, 8), (4, 6)),
//             MessorBarbarus => ((13, 16), (3, 14))
//         }
//     }
//     fn humidity(&self) -> i32 {
//         use Spec::*;
//         match self {
//             FormicaFusca => 10,
//             MyrmicaRubra => 20,
//             MessorBarbarus => 10
//         }
//     }
// }

// enum Area {
//     MiddleterraneanForest,
//     MiddleterraneanAcre
// }

// impl Area {
//     fn render(&self, s: &mut Screen) {
//         // 149, 125, 173
//         use Area::*;
//         s.text(self.name(), s.w/2, s.h/2 - 2, Align::Center, Color::Rgb { r: 206, g: 58, b: 132 }, None);
//         let (low, high) = self.temperature();
//         s.text(format!("Temperature {}~{}°C", low, high), s.w/2, s.h/2, Align::Center, Color::Rgb { r: 255, g: 179, b: 71 }, None);
//         let (low, high) = self.humidity();
//         s.text(format!("Humidity {}~{}%", low, high), s.w/2, s.h/2 + 1, Align::Center, Color::Rgb { r: 79, g: 185, b: 226 }, None);

//         match self {
//             MiddleterraneanForest => {
//                 s.text("- Moderate diversity -", s.w/2, s.h/2 + 3, Align::Center, Color::Rgb { r: 119, g: 221, b: 119 }, None);
//             }
//             MiddleterraneanAcre => {
//                 s.text("- Open space -", s.w/2, s.h/2 + 3, Align::Center, Color::Rgb { r: 119, g: 221, b: 119 }, None);
//             }
//         }
//     }
//     fn name(&self) -> String {
//         use Area::*;
//         match self {
//             MiddleterraneanForest => "Middleterranean Forest".into(),
//             MiddleterraneanAcre => "Middleterranean Acre".into(),
//         }
//     }
//     fn temperature(&self) -> (i32, i32) {
//         use Area::*;
//         match self {
//             MiddleterraneanForest => (-8, 28),
//             MiddleterraneanAcre => (-12, 32),
//         }
//     }
//     fn humidity(&self) -> (i32, i32) {
//         use Area::*;
//         match self {
//             MiddleterraneanForest => (10, 35),
//             MiddleterraneanAcre => (5, 30),
//         }
//     }
// }

// struct Game {
//     time: u64,
//     last: Instant,
//     update: Instant,
//     spec: Spec,
//     area: Area,
// }

// impl Game {
//     fn new(spec: Spec, area: Area) -> Self {
//         Self {
//             time: 0,
//             last: Instant::now(),
//             update: Instant::now(),
//             spec,
//             area
//         }
//     }
//     fn update(&mut self) {
//         self.time += self.last.elapsed().as_millis() as u64;
//         self.last = Instant::now();
//         if self.update.elapsed().as_millis() >= 100 {
//             self.update = Instant::now();
            
//         }
//     }
// }

// enum Stat {
//     Colony {
//         ants: Vec<Ant>,
        
//         dt: f32,     // +faster temperature change, -slower temperature change

//         // 10x  100x
//         // egg, food
//         nest: f32   // nestlvl = 1 + nest.sqrt().floor() as i32
//     },
//     Temperature {
//         curr: f32,
//         target: f32,
//     },
//     NestTemperature {
//         curr: f32,
//         target: f32,
//     },
//     Humidity {
//         curr: f32,
//         target: f32,
//     }
// }

// enum Ant {
//     Queen {
//         health: f32,
//         size: f32,
//         food: f32,
//         birth: u64,
//         next: u64,
//         delay: u64
//     },
//     Egg {
//         health: f32,
//         birth: u64,
//         ant: Box<Ant>
//     },
//     Worker { // performance = health * wpow * if food < 0.333 {0.5} else {1.0}
//         health: f32,
//         size: f32,
//         food: f32,
//         birth: u64,
//         eage: u64,
//         wpow: f32,
//         task: Task
//     }
// }

// #[derive(Clone, Copy, PartialEq, Eq)]
// enum Task {
//     Idle,
//     Exploring,
//     Eating,
//     Housecaring,
//     Building(Building),
// }

// #[derive(Clone, Copy, PartialEq, Eq)]
// enum Building {
//     Nest,
//     AirVents,
//     ThickerWalls
// }

// impl Task {
//     fn text(&self) -> String {
//         match self {
//             Task::Idle => "idle".into(),
//             Task::Exploring => "exploring".into(),
//             Task::Eating => "eating".into(),
//             Task::Housecaring => "housecaring".into(),
//             Task::Building(Building::Nest) => "upgrading nest".into(),
//             Task::Building(Building::AirVents) => "building air vents".into(),
//             Task::Building(Building::ThickerWalls) => "building thicker walls".into(),
//         }
//     }
// }

// struct Screen {
//     out: Stdout,
//     w: usize,
//     h: usize,
//     update: bool,
//     buff: Vec<(char, Color, Option<Attribute>)>,
// }

// impl Screen {
//     fn new() -> Self {
//         let mut stdout = stdout();
//         let (w, h) = terminal::size().unwrap();

//         stdout.execute(terminal::Clear(terminal::ClearType::All)).unwrap();
//         stdout.execute(terminal::SetSize(w, h)).unwrap();
//         stdout.execute(cursor::DisableBlinking).unwrap();
//         stdout.execute(cursor::Hide).unwrap();

//         Self {
//             out: stdout,
//             w: w as usize,
//             h: h as usize,
//             update: true,
//             buff: vec![(' ', Color::White, None); w as usize * h as usize],
//         }
//     }
//     fn render(&mut self) {
//         if self.update {
//             for y in 0..self.h {
//                 for x in 0..self.w {
//                     let k = y * self.w + x;
//                     if let Some(attr) = self.buff[k].2 {
//                         queue!(
//                             self.out,
//                             MoveTo(x as u16, y as u16),
//                             SetForegroundColor(self.buff[k].1),
//                             SetAttribute(attr),
//                             Print(self.buff[k].0),
//                             SetAttribute(Attribute::Reset)
//                         );
//                     } else {
//                         queue!(
//                             self.out,
//                             MoveTo(x as u16, y as u16),
//                             SetForegroundColor(self.buff[k].1),
//                             Print(self.buff[k].0),
//                             SetAttribute(Attribute::Reset)
//                         );
//                     }
//                     self.buff[k].0 = ' ';
//                 }
//             }
//             self.update = false;
//         }
//     }
//     fn text(&mut self, s: impl AsRef<str>, x: usize, y: usize, align: Align, color: Color, attr: Option<Attribute>) {
//         let s = s.as_ref();
//         let len = s.len();
//         for (k, c) in s.chars().enumerate() {
//             let k = match align {
//                 Align::Left => y * self.w + x + k,
//                 Align::Center => y * self.w + (x - len/2) + k,
//                 Align::Right => y * self.w + x - len + 1 + k
//             };
//             self.buff[k] = (c, color, attr);
//         }
//         self.update = true;
//     }
//     fn event(&mut self) -> Option<Event> {
//         if let Ok(true) = poll(Duration::from_millis(50)) {
//             Some(read().unwrap())
//         } else {
//             None
//         }
//     }
//     fn clear(&mut self) {
//         for p in &mut self.buff {
//             *p = (' ', Color::White, None);
//         }
//     }
// }

// enum Align {
//     Left,
//     Center,
//     Right
// }