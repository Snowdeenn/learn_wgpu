use super::DebugUi;

pub enum WidgetKind {
    Window,
    Header { default_open: bool },
    Group,
    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    Horizontal,
    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    Vertical,
}

pub struct WidgetBuilder<'a> {
    ctx: Option<&'a egui::Context>,
    parent_ui: Option<&'a mut egui::Ui>,
    title: String,
    kind: WidgetKind,
    size: Option<[f32; 2]>,
    resizable: bool,
    collapsible: bool,
}

impl<'a> WidgetBuilder<'a> {
    pub fn new_window(ctx: &'a egui::Context, title: impl Into<String>) -> Self {
        Self {
            ctx: Some(ctx),
            parent_ui: None,
            title: title.into(),
            kind: WidgetKind::Window,
            size: None,
            resizable: true,
            collapsible: true,
        }
    }

    pub fn new_child(parent_ui: &'a mut egui::Ui, title: impl Into<String>) -> Self {
        Self {
            ctx: None,
            parent_ui: Some(parent_ui),
            title: title.into(),
            kind: WidgetKind::Group,
            size: None,
            resizable: false,
            collapsible: false,
        }
    }

    // --- Méthodes du Builder ---

    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn kind(mut self, kind: WidgetKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn window(mut self) -> Self {
        self.kind = WidgetKind::Window;
        self
    }

    pub fn header(mut self, default_open: bool) -> Self {
        self.kind = WidgetKind::Header { default_open };
        self
    }

    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn group(mut self) -> Self {
        self.kind = WidgetKind::Group;
        self
    }

    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn horizontal(mut self) -> Self {
        self.kind = WidgetKind::Horizontal;
        self
    }

    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn vertical(mut self) -> Self {
        self.kind = WidgetKind::Vertical;
        self
    }

    pub fn size(mut self, size: [f32; 2]) -> Self {
        self.size = Some(size);
        self
    }

    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    pub fn show<R>(self, add_contents: impl FnOnce(&mut DebugUi) -> R) -> Option<R> {
        match self.kind {
            WidgetKind::Window => {
                let ctx = self.ctx?;
                let mut win = egui::Window::new(&self.title)
                    .resizable(self.resizable)
                    .collapsible(self.collapsible);

                if let Some(s) = self.size {
                    win = win.default_size(s);
                }

                let mut result = None;
                win.show(ctx, |ui| {
                    let mut widget = DebugUi::new(ui);
                    result = Some(add_contents(&mut widget));
                });
                result
            }
            WidgetKind::Header { default_open } => {
                let ui = self.parent_ui?;
                let mut result = None;
                egui::CollapsingHeader::new(egui::RichText::new(&self.title).strong())
                    .default_open(default_open)
                    .show(ui, |ui| {
                        let mut widget = DebugUi::new(ui);
                        result = Some(add_contents(&mut widget));
                    });
                ui.separator();
                result
            }
            WidgetKind::Group => {
                let ui = self.parent_ui?;
                let mut result = None;
                ui.group(|ui| {
                    let mut widget = DebugUi::new(ui);
                    result = Some(add_contents(&mut widget));
                });
                result
            }
            WidgetKind::Horizontal => {
                let ui = self.parent_ui?;
                let mut result = None;
                ui.horizontal(|ui| {
                    let mut widget = DebugUi::new(ui);
                    result = Some(add_contents(&mut widget));
                });
                result
            }
            WidgetKind::Vertical => {
                let ui = self.parent_ui?;
                let mut result = None;
                ui.vertical(|ui| {
                    let mut widget = DebugUi::new(ui);
                    result = Some(add_contents(&mut widget));
                });
                result
            }
        }
    }
}
