use super::WidgetBuilder;

pub struct DebugUi<'a> {
    pub ui: &'a mut egui::Ui,
}

impl<'a> DebugUi<'a> {
    pub fn new(ui: &'a mut egui::Ui) -> Self {
        Self { ui }
    }

    pub fn create_widget<'b>(&'b mut self, title: impl Into<String>) -> WidgetBuilder<'b> {
        WidgetBuilder::new_child(self.ui, title)
    }

    pub fn button(&mut self, text: impl Into<String>) -> egui::Response {
        self.ui.button(text.into())
    }

    pub fn toggle(&mut self, label: &str, value: &mut bool) -> egui::Response {
        self.ui.checkbox(value, label)
    }

    pub fn checkbox(&mut self, label: &str) -> bool {
        let id = self.ui.make_persistent_id(label);
        let mut state = self
            .ui
            .ctx()
            .data_mut(|d| d.get_temp::<bool>(id).unwrap_or(false));
        self.ui.checkbox(&mut state, label);
        self.ui.ctx().data_mut(|d| d.insert_temp(id, state));
        state
    }

    pub fn slider(
        &mut self,
        label: &str,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
    ) -> egui::Response {
        self.ui.add(egui::Slider::new(value, range).text(label))
    }

    pub fn metric(&mut self, label: &str, value: impl std::fmt::Display) {
        self.ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{label} :")).strong());
            ui.label(value.to_string());
        });
    }

    pub fn metric_colored(
        &mut self,
        label: &str,
        value: impl std::fmt::Display,
        color: egui::Color32,
    ) {
        self.ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{label} :")).strong());
            ui.colored_label(color, value.to_string());
        });
    }
}
