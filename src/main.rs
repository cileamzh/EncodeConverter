#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use eframe::{
    App,
    egui::{self, ViewportBuilder},
    icon_data::from_png_bytes,
};
use encoding_rs::*;
use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
    thread,
};

static FONT: &[u8] = include_bytes!("../font.ttf");
static ICON: &[u8] = include_bytes!("../tlogo.png");

/* ======================= 编码表 ======================= */
/*
    [0] encoding_rs::Encoding
    [1] 显示名称
*/
type EncodingItem = (&'static Encoding, &'static str);

static ENCODINGS: &[EncodingItem] = &[
    (UTF_8, "UTF-8"),
    (UTF_16LE, "UTF-16LE"),
    (UTF_16BE, "UTF-16BE"),
    (GBK, "GBK"),
    (GB18030, "GB18030"),
    (BIG5, "BIG5"),
    (SHIFT_JIS, "Shift_JIS"),
    (EUC_JP, "EUC-JP"),
    (ISO_2022_JP, "ISO-2022-JP"),
    (EUC_KR, "EUC-KR"),
    (WINDOWS_1250, "Windows-1250"),
    (WINDOWS_1251, "Windows-1251"),
    (WINDOWS_1252, "Windows-1252"),
    (WINDOWS_1253, "Windows-1253"),
    (WINDOWS_1254, "Windows-1254"),
    (WINDOWS_1255, "Windows-1255"),
    (WINDOWS_1256, "Windows-1256"),
    (WINDOWS_1257, "Windows-1257"),
    (WINDOWS_1258, "Windows-1258"),
    (ISO_8859_2, "ISO-8859-2"),
    (ISO_8859_3, "ISO-8859-3"),
    (ISO_8859_4, "ISO-8859-4"),
    (ISO_8859_5, "ISO-8859-5"),
    (ISO_8859_6, "ISO-8859-6"),
    (ISO_8859_7, "ISO-8859-7"),
    (ISO_8859_8, "ISO-8859-8"),
    (ISO_8859_10, "ISO-8859-10"),
    (ISO_8859_13, "ISO-8859-13"),
    (ISO_8859_14, "ISO-8859-14"),
    (ISO_8859_15, "ISO-8859-15"),
    (ISO_8859_16, "ISO-8859-16"),
    (MACINTOSH, "Macintosh"),
    (KOI8_R, "KOI8-R"),
    (KOI8_U, "KOI8-U"),
    (IBM866, "IBM866"),
];

/* ======================= 语言 ======================= */
#[derive(Clone, Copy, PartialEq)]
enum Language {
    Zh,
    En,
}

fn t(key: &str, lang: Language) -> &str {
    match lang {
        Language::Zh => match key {
            "text" => "文本转码",
            "file" => "文件转码",
            "from" => "来源编码",
            "to" => "目标编码",
            "start" => "开始转码",
            "input" => "输入文本",
            "output" => "输出结果",
            "select_input" => "选择输入文件",
            "select_output" => "选择输出文件",
            "working" => "正在转码...",
            "idle" => "暂无状态",
            _ => key,
        },
        Language::En => match key {
            "text" => "Text",
            "file" => "File",
            "from" => "From",
            "to" => "To",
            "start" => "Start",
            "input" => "Input Text",
            "output" => "Output",
            "select_input" => "Select Input File",
            "select_output" => "Select Output File",
            "working" => "Working...",
            "idle" => "Idle",
            _ => key,
        },
    }
}

/* ======================= 模式 ======================= */
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Text,
    File,
}

/* ======================= 转码逻辑 ======================= */
fn transcode_text(input: &str, from: usize, to: usize) -> String {
    let (from_enc, _) = ENCODINGS[from];
    let (to_enc, _) = ENCODINGS[to];

    let (decoded, _, _) = from_enc.decode(input.as_bytes());
    let (encoded, _, _) = to_enc.encode(&decoded);

    String::from_utf8_lossy(&encoded).to_string()
}

fn transcode_file(input: PathBuf, output: PathBuf, from: usize, to: usize) -> String {
    let data = match std::fs::read(&input) {
        Ok(v) => v,
        Err(e) => return e.to_string(),
    };

    let (from_enc, _) = ENCODINGS[from];
    let (to_enc, _) = ENCODINGS[to];

    let (decoded, _, _) = from_enc.decode(&data);
    let (encoded, _, _) = to_enc.encode(&decoded);

    match std::fs::write(&output, encoded) {
        Ok(_) => format!("Done: {}", output.display()),
        Err(e) => e.to_string(),
    }
}

/* ======================= App ======================= */
struct CodeTransApp {
    lang: Language,
    mode: Mode,

    from_idx: usize,
    to_idx: usize,

    input_text: String,
    output_text: String,

    input_file: Option<PathBuf>,
    output_file: Option<PathBuf>,

    status: String,

    rx: Option<mpsc::Receiver<String>>,
}

impl Default for CodeTransApp {
    fn default() -> Self {
        Self {
            lang: Language::Zh,
            mode: Mode::Text,
            from_idx: 0,
            to_idx: 3, // UTF-8 -> GBK
            input_text: String::new(),
            output_text: String::new(),
            input_file: None,
            output_file: None,
            status: t("idle", Language::Zh).into(),
            rx: None,
        }
    }
}

impl App for CodeTransApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("中文").clicked() {
                    self.lang = Language::Zh;
                }
                if ui.button("EN").clicked() {
                    self.lang = Language::En;
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.mode, Mode::Text, t("text", self.lang));
                ui.selectable_value(&mut self.mode, Mode::File, t("file", self.lang));
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.label(t("from", self.lang));
                encoding_combo(ui, "from", &mut self.from_idx);
                ui.label(t("to", self.lang));
                encoding_combo(ui, "to", &mut self.to_idx);
            });

            ui.separator();

            match self.mode {
                Mode::Text => self.ui_text(ui),
                Mode::File => self.ui_file(ui),
            }

            if let Some(rx) = &self.rx {
                if let Ok(msg) = rx.try_recv() {
                    match self.mode {
                        Mode::Text => self.output_text = msg,
                        Mode::File => self.status = msg,
                    }
                }
            }
        });
    }
}

/* ======================= UI ======================= */
impl CodeTransApp {
    fn ui_text(&mut self, ui: &mut egui::Ui) {
        ui.label(t("input", self.lang));
        ui.text_edit_multiline(&mut self.input_text);

        if ui.button(t("start", self.lang)).clicked() {
            let (tx, rx) = mpsc::channel();
            let input = self.input_text.clone();
            let from = self.from_idx;
            let to = self.to_idx;
            self.rx = Some(rx);

            thread::spawn(move || {
                tx.send(transcode_text(&input, from, to)).ok();
            });
        }

        ui.separator();
        ui.label(t("output", self.lang));
        ui.text_edit_multiline(&mut self.output_text);
    }

    fn ui_file(&mut self, ui: &mut egui::Ui) {
        if ui.button(t("select_input", self.lang)).clicked() {
            self.input_file = rfd::FileDialog::new().pick_file();
        }
        if ui.button(t("select_output", self.lang)).clicked() {
            self.output_file = rfd::FileDialog::new().save_file();
        }

        if ui.button(t("start", self.lang)).clicked() {
            if let (Some(i), Some(o)) = (self.input_file.clone(), self.output_file.clone()) {
                self.status = t("working", self.lang).into();
                let (tx, rx) = mpsc::channel();
                let from = self.from_idx;
                let to = self.to_idx;
                self.rx = Some(rx);

                thread::spawn(move || {
                    tx.send(transcode_file(i, o, from, to)).ok();
                });
            }
        }

        ui.separator();
        ui.label(&self.status);
    }
}

/* ======================= 编码下拉 ======================= */
fn encoding_combo(ui: &mut egui::Ui, id: &str, value: &mut usize) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(ENCODINGS[*value].1)
        .show_ui(ui, |ui| {
            for (i, (_, label)) in ENCODINGS.iter().enumerate() {
                ui.selectable_value(value, i, *label);
            }
        });
}

/* ======================= 字体 ======================= */
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("cjk".into(), Arc::new(egui::FontData::from_static(FONT)));
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "cjk".into());
    fonts
        .families
        .get_mut(&egui::FontFamily::Monospace)
        .unwrap()
        .insert(0, "cjk".into());
    ctx.set_fonts(fonts);
}

/* ======================= main ======================= */
fn main() -> Result<(), eframe::Error> {
    let icon = from_png_bytes(ICON).unwrap();

    let opts = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "EncodeConverter",
        opts,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(CodeTransApp::default()))
        }),
    )
}
