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

static FONT: &[u8] = include_bytes!("../font.ttf"); // 中文字体
static ICON: &[u8] = include_bytes!("../tlogo.png"); // 应用图标

/* ======================= 语言 ======================= */
#[derive(Debug, Clone, Copy, PartialEq)]
enum Language {
    Zh,
    En,
}

fn t(key: &str, lang: Language) -> &str {
    match lang {
        Language::Zh => match key {
            "text_mode" => "文本转码",
            "file_mode" => "文件转码",
            "from" => "来源编码",
            "to" => "目标编码",
            "input_text" => "输入文本",
            "output_text" => "输出结果",
            "start" => "开始转码",
            "select_input" => "选择输入文件",
            "select_output" => "选择输出文件",
            "status_none" => "暂无状态",
            "transcoding..." => "正在转码...",
            _ => key,
        },
        Language::En => match key {
            "text_mode" => "Text Transcode",
            "file_mode" => "File Transcode",
            "from" => "From",
            "to" => "To",
            "input_text" => "Input Text",
            "output_text" => "Output Text",
            "start" => "Start Transcode",
            "select_input" => "Select Input File",
            "select_output" => "Select Output File",
            "status_none" => "No Status",
            "transcoding..." => "Transcoding...",
            _ => key,
        },
    }
}

/* ======================= 数据模型 ======================= */
#[derive(Debug, Clone, Copy, PartialEq)]
enum TransMode {
    Text,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Encoding {
    Utf8,
    Gbk,
    Big5,
    Iso88592,
}

impl Encoding {
    fn label(self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Gbk => "GBK",
            Encoding::Big5 => "BIG5",
            Encoding::Iso88592 => "ISO-8859-2",
        }
    }

    fn encoding(self) -> &'static encoding_rs::Encoding {
        match self {
            Encoding::Utf8 => UTF_8,
            Encoding::Gbk => GBK,
            Encoding::Big5 => BIG5,
            Encoding::Iso88592 => ISO_8859_2,
        }
    }
}

/* ======================= 转码逻辑 ======================= */
fn transcode_text(input: &str, from: Encoding, to: Encoding) -> Result<String, String> {
    let (decoded, _, _) = from.encoding().decode(input.as_bytes());
    let (encoded, _, _) = to.encoding().encode(&decoded);
    Ok(String::from_utf8_lossy(&encoded).to_string())
}

fn transcode_file(
    input: &PathBuf,
    output: &PathBuf,
    from: Encoding,
    to: Encoding,
) -> Result<(), String> {
    let data = std::fs::read(input).map_err(|e| e.to_string())?;
    let (decoded, _, _) = from.encoding().decode(&data);
    let (encoded, _, _) = to.encoding().encode(&decoded);
    std::fs::write(output, encoded).map_err(|e| e.to_string())?;
    Ok(())
}

/* ======================= App 状态 ======================= */
pub struct CodeTranserApp {
    lang: Language,
    mode: TransMode,
    from: Encoding,
    to: Encoding,

    input_text: String,
    output_text: String,

    input_file: Option<PathBuf>,
    output_file: Option<PathBuf>,
    status: String,

    sender: Option<mpsc::Sender<String>>,
    receiver: Option<mpsc::Receiver<String>>,
}

impl Default for CodeTranserApp {
    fn default() -> Self {
        Self {
            lang: Language::Zh,
            mode: TransMode::Text,
            from: Encoding::Utf8,
            to: Encoding::Gbk,
            input_text: String::new(),
            output_text: String::new(),
            input_file: None,
            output_file: None,
            status: t("status_none", Language::Zh).to_string(),
            sender: None,
            receiver: None,
        }
    }
}

/* ======================= UI ======================= */
impl App for CodeTranserApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // 语言切换
            ui.horizontal(|ui| {
                if ui.button("中文").clicked() {
                    self.lang = Language::Zh;
                }
                if ui.button("EN").clicked() {
                    self.lang = Language::En;
                }
            });

            ui.separator();

            // 模式选择
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.mode, TransMode::Text, t("text_mode", self.lang));
                ui.selectable_value(&mut self.mode, TransMode::File, t("file_mode", self.lang));
            });

            ui.separator();

            // 编码选择
            ui.horizontal(|ui| {
                ui.label(t("from", self.lang));
                encoding_combo(ui, "from", &mut self.from);
                ui.label(t("to", self.lang));
                encoding_combo(ui, "to", &mut self.to);
            });

            ui.separator();

            match self.mode {
                TransMode::Text => self.ui_text_mode(ui),
                TransMode::File => self.ui_file_mode(ui),
            }

            // 异步结果检查
            if let Some(rx) = &self.receiver {
                if let Ok(res) = rx.try_recv() {
                    match self.mode {
                        TransMode::Text => self.output_text = res,
                        TransMode::File => self.status = res,
                    }
                }
            }
        });
    }
}

/* ======================= 子 UI ======================= */
impl CodeTranserApp {
    fn ui_text_mode(&mut self, ui: &mut egui::Ui) {
        ui.label(t("input_text", self.lang));
        ui.text_edit_multiline(&mut self.input_text);

        if ui.button(t("start", self.lang)).clicked() {
            let input = self.input_text.clone();
            let from = self.from;
            let to = self.to;
            let (tx, rx) = mpsc::channel();
            self.sender = Some(tx.clone());
            self.receiver = Some(rx);

            thread::spawn(move || {
                let out = transcode_text(&input, from, to).unwrap_or_else(|e| e);
                tx.send(out).ok();
            });
        }

        ui.separator();
        ui.label(t("output_text", self.lang));
        ui.text_edit_multiline(&mut self.output_text);
    }

    fn ui_file_mode(&mut self, ui: &mut egui::Ui) {
        if ui.button(t("select_input", self.lang)).clicked() {
            self.input_file = rfd::FileDialog::new().pick_file();
        }
        if let Some(path) = &self.input_file {
            ui.label(format!("Input: {}", path.display()));
        }

        if ui.button(t("select_output", self.lang)).clicked() {
            self.output_file = rfd::FileDialog::new()
                .set_file_name("output.txt")
                .save_file();
        }
        if let Some(path) = &self.output_file {
            ui.label(format!("Output: {}", path.display()));
        }

        if ui.button(t("start", self.lang)).clicked() {
            if let (Some(input), Some(output)) = (&self.input_file, &self.output_file) {
                self.status = t("transcoding...", self.lang).to_string();
                let input = input.clone();
                let output = output.clone();
                let from = self.from;
                let to = self.to;
                let (tx, rx) = mpsc::channel();
                self.sender = Some(tx.clone());
                self.receiver = Some(rx);

                thread::spawn(move || {
                    let res = transcode_file(&input, &output, from, to)
                        .map(|_| format!("Transcode finished: {}", output.display()))
                        .unwrap_or_else(|e| format!("Error: {}", e));
                    tx.send(res).ok();
                });
            } else {
                self.status = "Please select input and output files".to_string();
            }
        }

        ui.separator();
        ui.label(&self.status);
    }
}

/* ======================= 编码选择 ======================= */
fn encoding_combo(ui: &mut egui::Ui, id: &str, value: &mut Encoding) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(value.label())
        .show_ui(ui, |ui| {
            ui.selectable_value(value, Encoding::Utf8, "UTF-8");
            ui.selectable_value(value, Encoding::Gbk, "GBK");
            ui.selectable_value(value, Encoding::Big5, "BIG5");
            ui.selectable_value(value, Encoding::Iso88592, "ISO-8859-2");
        });
}

/* ======================= 中文字体 ======================= */
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "cjk".to_owned(),
        Arc::new(egui::FontData::from_static(FONT)),
    );

    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "cjk".to_owned());

    fonts
        .families
        .get_mut(&egui::FontFamily::Monospace)
        .unwrap()
        .insert(0, "cjk".to_owned());

    ctx.set_fonts(fonts);
}

/* ======================= main ======================= */

fn main() -> Result<(), eframe::Error> {
    let icon = from_png_bytes(ICON).expect("icon err");
    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "EncodeConventer",
        native_options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(CodeTranserApp::default()))
        }),
    )
}
