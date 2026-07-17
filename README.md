# s3dm

**S3 Desktop Manager** — 一个跨平台的 S3 对象存储图形化管理客户端，使用 Rust 编写。

基于 [`iced`](https://iced.rs/) 构建的桌面应用，通过 [`aws-sdk-s3`](https://crates.io/crates/aws-sdk-s3) 连接 S3 兼容的对象存储服务，方便浏览和管理存储桶、文件夹与对象，并支持文件的上传与下载。

## 功能特性

- 连接并管理多个 S3 兼容存储服务（支持自定义 endpoint）
- 浏览存储桶（bucket）、目录与对象
- 上传 / 下载文件
- 跨平台（Linux / macOS / Windows），提供原生桌面体验
- 国际化的界面（i18n）

## 项目结构

采用 Cargo workspace 组织，分为三个 crate：

| Crate                | 说明                                        |
| -------------------- | ------------------------------------------- |
| `crates/s3dm-config` | 连接配置与本地配置管理                      |
| `crates/s3dm-core`   | 核心逻辑：S3 客户端、桶/对象操作封装        |
| `crates/s3dm-gui`    | 基于 `iced` 的图形界面（含应用图标与 i18n） |

## 构建与运行

需要安装 Rust 工具链（建议较新版本以支持 `edition = "2024"`）。

```bash
# 构建
cargo build --release

# 运行
cargo run
```

## 打包

项目已配置 `cargo-deb` 与 `cargo-generate-rpm`，可分别生成 `.deb` 与 `.rpm` 包：

```bash
cargo deb
cargo generate-rpm
```

## 图标来源

应用图标来源于 [OpenSVG](https://opensvg.dev/icons)。

## 安全说明

连接配置（含 Access Key / Secret Key）以明文 JSON 存储在用户配置目录下：

- Linux / macOS：`~/.config/s3dm/connections.json`
- Windows：`%APPDATA%\s3dm\connections.json`

保存时文件权限会被限制为仅当前用户可读写（Unix 下为 `0600`）。**请勿将配置文件共享或提交到仓库**，并建议在系统层面做好磁盘加密与访问控制。

## 许可证

[MIT](./LICENSE)
