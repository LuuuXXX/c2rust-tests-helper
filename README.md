# c2rust-tests-helper

[`c2rust-demo`](https://github.com/LuuuXXX/c2rust-demo) 翻译输出分析工具。
用于从 `c2rust-demo` 生成的接口报告中提取符号，并分析 Rust `#[test]` 对接口的覆盖关系。

## 安装

### 前置条件

- Rust 工具链（stable）
- 已初始化至少一个 feature workspace 的 `c2rust-demo` 项目

### 构建

```bash
git clone https://github.com/LuuuXXX/c2rust-tests-helper.git
cd c2rust-tests-helper
cargo build --release
```

编译产物位于 `target/release/c2rust-tests-helper`。

可将其加入 `PATH` 以便全局调用：

```bash
export PATH="$PWD/target/release:$PATH"
```

## 核心功能

| 命令 | 说明 |
|---|---|
| `interface` | 读取接口报告并输出全部接口符号（函数 + 变量） |
| `scan` | 扫描 Rust `#[test]`，分类 ST/DT，输出测试-接口匹配表 |
| `coverage` | 按接口符号输出 ST/DT 覆盖矩阵（Markdown） |

## 核心输入

支持两种接口报告：

- `meta/init-interface-report.md`（`c2rust-demo init` 生成）
- `meta/merge-interface-report.md`（`c2rust-demo merge` 生成）

当不传 `--report` 时，按以下顺序自动 fallback：

1. `meta/init-interface-report.md`
2. `meta/merge-interface-report.md`
3. `.c2rust/<feature>/meta/init-interface-report.md`（按 feature 名字排序）
4. `.c2rust/<feature>/meta/merge-interface-report.md`（按 feature 名字排序）

若都不存在则报错；若显式传入 `--report <path>`，则直接使用该路径。

## 输出文件

`scan` 和 `coverage` 会在打印 stdout 的同时写入文件：

| 命令 | 默认输出文件 |
|---|---|
| `scan` | `<report目录>/test-scan-report.md` |
| `coverage` | `<report目录>/coverage-report.md` |

默认情况下（使用 `meta/...-interface-report.md`），输出为：

- `meta/test-scan-report.md`
- `meta/coverage-report.md`

可通过 `--output <path>` 覆盖默认输出路径。

## 推荐工作流

```bash
# 运行 c2rust-demo init 或 merge 后，在 feature 根目录执行：

# 1. 查看接口清单
c2rust-tests-helper interface

# 2. 扫描 Rust 测试，输出 meta/test-scan-report.md
c2rust-tests-helper scan --dir .c2rust/default/rust

# 3. 生成覆盖矩阵，输出 meta/coverage-report.md
c2rust-tests-helper coverage --dir .c2rust/default/rust
```

## 参数说明

### `interface`

- `-r, --report <path>`：接口报告路径（可选，不传时自动 fallback）

### `scan`

- `-r, --report <path>`：接口报告路径（可选，不传时自动 fallback）
- `-d, --dir <path>`：Rust 源码扫描根目录（默认 `.`）
- `-o, --output <path>`：扫描报告输出路径（可选）

### `coverage`

- `-r, --report <path>`：接口报告路径（可选，不传时自动 fallback）
- `-d, --dir <path>`：Rust 源码扫描根目录（默认 `.`）
- `-o, --output <path>`：覆盖矩阵输出路径（可选）

## meta 目录产物全览

```text
meta/
├── init-interface-report.md      # c2rust-demo init 生成（可选）
├── merge-interface-report.md     # c2rust-demo merge 生成（可选）
├── test-scan-report.md           # c2rust-tests-helper scan 生成
└── coverage-report.md            # c2rust-tests-helper coverage 生成
```
