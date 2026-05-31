use crate::app::widgets::progress::job_progress_row;
use crate::core::task::manager::SharedJob;

/// 渲染任务面板（作为浮动窗口）
pub fn render_task_panel(
    ctx: &egui::Context,
    show: &mut bool,
    jobs: &[SharedJob],
    mut on_cancel: impl FnMut(u64),
    mut on_clear: impl FnMut(),
) {
    if !*show {
        return;
    }

    let mut window_open = true;

    egui::Window::new("📋 Task Queue")
        .open(&mut window_open)
        .resizable(true)
        .collapsible(true)
        .anchor(egui::Align2::RIGHT_TOP, [-10.0, 30.0])
        .min_width(400.0)
        .default_height(300.0)
        .show(ctx, |ui| {
            render_task_list(ui, jobs, &mut on_cancel, &mut on_clear);
        });

    if !window_open {
        *show = false;
    }
}

fn render_task_list(
    ui: &mut egui::Ui,
    jobs: &[SharedJob],
    on_cancel: &mut impl FnMut(u64),
    on_clear: &mut impl FnMut(),
) {
    if jobs.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label("No tasks. Add files and press Run to start.");
        });
        return;
    }

    // 头部
    ui.horizontal(|ui| {
        ui.heading("Tasks");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let has_completed = jobs.iter().any(|j| {
                j.lock().unwrap().status.is_terminal()
            });
            if has_completed {
                if ui.button("Clear Completed").clicked() {
                    on_clear();
                }
            }
        });
    });

    ui.separator();

    // 任务列表
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for job in jobs {
                let (id, input_name, status_str, progress, elapsed_secs, is_active) = {
                    let j = job.lock().unwrap();
                    (
                        j.id,
                        j.input_name(),
                        j.status.display().to_string(),
                        j.progress,
                        j.elapsed().map(|d| d.as_secs_f64()),
                        !j.status.is_terminal(),
                    )
                };

                ui.horizontal(|ui| {
                    // 文件名 + 状态 + 进度
                    job_progress_row(
                        ui,
                        &input_name,
                        &status_str,
                        progress,
                        elapsed_secs,
                    );

                    // 取消按钮（仅活跃任务）
                    if is_active {
                        if ui.button("✕").clicked() {
                            on_cancel(id);
                        }
                    }
                });

                ui.add_space(2.0);
            }
        });
}

/// 嵌入式任务列表（非窗口模式，直接嵌入到主页面中）
pub fn render_task_list_embedded(
    ui: &mut egui::Ui,
    jobs: &[SharedJob],
    mut on_cancel: impl FnMut(u64),
    mut on_clear: impl FnMut(),
) {
    ui.separator();
    ui.collapsing("📋 Task Queue", |ui| {
        render_task_list(ui, jobs, &mut on_cancel, &mut on_clear);
    });
}
