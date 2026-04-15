A convenient media file format converter, based on ffmpeg, almost supports all formats of video/audio/images

media_tool/
├── Cargo.toml
├── Cargo.lock
├── assets/                      # 静态资源（图标、预设配置等）
├── src/
│   ├── main.rs                  # 程序入口：初始化 egui 窗口、tokio 运行时、通道
│   ├── app/                     # UI 层 (egui 相关，不含业务逻辑)
│   │   ├── mod.rs               # 实现 eframe::App trait，驱动每帧绘制
│   │   ├── state.rs             # 应用状态聚合 (AppState)，无 GUI 依赖
│   │   ├── ui/                  # 各个面板的纯绘制函数
│   │   │   ├── mod.rs
│   │   │   ├── menu.rs          # 顶部菜单栏
│   │   │   ├── media_library.rs # 媒体库文件列表
│   │   │   ├── task_panel.rs    # 任务队列与进度
│   │   │   ├── preview.rs       # 预览与播放控制
│   │   │   ├── settings.rs      # 设置对话框
│   │   │   └── dialogs.rs       # 文件选择、确认等模态框
│   │   └── widgets/             # 自定义 egui 控件
│   │       ├── mod.rs
│   │       ├── timeline.rs      # 剪辑时间轴
│   │       ├── filter_editor.rs # 滤镜链编辑器
│   │       └── progress.rs      # 进度条与任务状态徽章
│   ├── core/                    # 核心业务逻辑 (零 GUI 依赖)
│   │   ├── mod.rs
│   │   ├── task/                # 任务管理
│   │   │   ├── mod.rs
│   │   │   ├── manager.rs       # 任务队列、并发控制、状态轮询
│   │   │   ├── job.rs           # 任务定义与状态机
│   │   │   └── scheduler.rs     # 并发调度器 (基于 Semaphore)
│   │   ├── processor/           # 媒体处理逻辑
│   │   │   ├── mod.rs
│   │   │   ├── ffmpeg.rs        # FFmpeg 命令构建器 (纯参数生成)
│   │   │   ├── converter.rs     # 格式转换逻辑
│   │   │   ├── clipper.rs       # 精确剪辑逻辑
│   │   │   ├── slicer.rs        # HLS/DASH 切片逻辑
│   │   │   └── renderer.rs      # 滤镜渲染合成逻辑
│   │   ├── player/              # 播放器控制
│   │   │   ├── mod.rs
│   │   │   ├── mpv.rs           # MPV IPC 命令构建与响应解析
│   │   │   └── window.rs        # 播放窗口生命周期管理
│   │   └── utils/               # 通用工具
│   │       ├── config.rs        # 配置读写 (serde)
│   │       ├── scanner.rs       # 文件系统扫描与媒体识别
│   │       └── metadata.rs      # 调用 ffprobe 提取元数据
│   ├── services/                # 服务层 (异步执行，通过通道返回结果)
│   │   ├── mod.rs
│   │   ├── ffmpeg_service.rs    # 启动 FFmpeg 进程，解析进度，发送 UiMessage
│   │   ├── mpv_service.rs       # 启动 MPV 进程，IPC 通信，发送播放事件
│   │   └── file_service.rs      # 异步文件读写 (保存/加载任务状态)
│   └── channels/                # 跨线程通信定义
│       ├── mod.rs
│       └── messages.rs          # UiMessage 枚举 (进度、完成、错误、播放事件)
└── build.rs                     # (可选) 编译时嵌入资源
