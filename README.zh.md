# rs_lvgl

[English](https://github.com/umbrella22/rs_lvgl/blob/main/README.md) · **简体中文**

面向 [LVGL](https://lvgl.io/) 9.6 的 Rust 绑定与声明式 UI 库。

可以直接使用 Rust 封装的 LVGL 控件，也可以用 `rs_lvgl_ui` 描述界面，通过消息更新状态。
开发时用 SDL2 在桌面运行界面，再接入设备的显示与输入驱动。

![rs_lvgl 组件工作台](https://raw.githubusercontent.com/umbrella22/rs_lvgl/main/docs/images/studio-overview.png)

## 运行示例

组件工作台包含基础控件、图表、文本输入、布局、多媒体和导航，支持明暗主题。
也可以浏览[组件图集](https://github.com/umbrella22/rs_lvgl/blob/main/docs/component-studio.md)或从[小型示例](https://github.com/umbrella22/rs_lvgl/blob/main/examples/README.md)入手。

需要 Rust、C/C++ 编译器、libclang、SDL2 和 pkg-config。

```sh
# Debian / Ubuntu
sudo apt-get install build-essential clang libclang-dev pkg-config libsdl2-dev

# macOS：先安装 Xcode Command Line Tools
brew install sdl2 pkg-config
```

在仓库根目录运行：

```sh
cargo run -p rs_lvgl_ui --example studio --features showcase
```

## 接入项目

| Crate | 用途 |
| --- | --- |
| [`rs_lvgl_ui`](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_ui/README.md) | 声明式界面、类型化事件、主题和可复用组件 |
| [`rs_lvgl`](https://github.com/umbrella22/rs_lvgl/tree/main/rs_lvgl) | 控件、显示、输入、样式和计时器的 Rust 封装 |
| [`rs_lvgl_sys`](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_sys/README.md) | 原始 LVGL 绑定、原生配置、自定义 C 源码与字体 |
| [`rs_lvgl_codegen`](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_codegen/README.md) | 构建时使用的控件封装生成器 |

从 crates.io 添加依赖：

```toml
[dependencies]
rs_lvgl = { version = "0.1.0", default-features = false, features = ["alloc"] }
rs_lvgl_ui = { version = "0.1.0" }
```

`rs_lvgl` 和 `rs_lvgl_ui` 支持 `no_std`。在嵌入式设备上使用 UI 层时，
需要提供分配器以及对应的显示和输入驱动。

## 描述界面

`App` 保存应用状态，`view` 返回节点树，输入通过消息交给 `update` 处理。
`Ui` 负责将变化应用到 LVGL 控件。

```rust
use rs_lvgl_ui::{App, Key, Node, Style, Theme};

#[derive(Clone)]
enum Msg {
    Increment,
}

struct Counter {
    value: u32,
}

impl App for Counter {
    type Msg = Msg;

    fn update(&mut self, _: Msg) {
        self.value += 1;
    }

    fn view(&self, theme: &Theme) -> Node<Msg> {
        Node::column()
            .style(Style::default().gap(12))
            .child(Node::label(format!("Count: {}", self.value)))
            .child(
                Node::button("Add one")
                    .key(Key::Named("increment"))
                    .style(Style::accent(theme))
                    .on_click(Msg::Increment),
            )
    }
}
```

[完整的计数器示例](https://github.com/umbrella22/rs_lvgl/blob/main/.agents/skills/rs-lvgl-ui/references/counter.rs)包含显示初始化和事件循环；
布局、样式和事件的用法见 [UI 指南](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_ui/README.md)。

## 选择控件与字体

关闭默认 feature 后，可以按需选择控件和字体：

```toml
rs_lvgl_ui = { version = "0.1.0", default-features = false, features = [
    "alloc", "components", "widget-slider", "widget-switch", "font-montserrat-16"
] }
```

标签、按钮、行列布局和网格布局始终可用。
二维码、条码、GIF、Lottie、拼音输入和 CJK 字体可单独启用。
完整列表见[控件选择](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_ui/README.md#choosing-widgets)。

## 文档

- [UI 指南](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_ui/README.md)：组件、状态、样式与输入。
- [原生集成](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_sys/README.md)：`lv_conf.h`、自定义 C 源码与字体。
- [测试指南](https://github.com/umbrella22/rs_lvgl/blob/main/docs/testing.md)：测试命令与模拟器检查。
- [更新记录](https://github.com/umbrella22/rs_lvgl/blob/main/CHANGELOG.md)。

## AI 编程助手

仓库提供 `rs-lvgl-ui` 和 `rs-lvgl-native` 两个 skill，包含 API 示例和集成说明，供编程助手使用。
安装命令：

```sh
python3 scripts/install-skills.py --dest ~/.agents/skills
```

项目内安装方式与用法见 [skill 配置说明](https://github.com/umbrella22/rs_lvgl/blob/main/docs/ai-skills.md)。

## 许可证

Rust 代码采用 [MIT](https://github.com/umbrella22/rs_lvgl/blob/main/LICENSE) 许可证，感谢 Rafael Carício、Nia Espera 及原始绑定的其他贡献者。
LVGL、ThorVG 和其他随仓库提供的依赖遵循各自的许可证。
