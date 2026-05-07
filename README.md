# c2rust-tests-helper

[`c2rust-demo`](https://github.com/LuuuXXX/c2rust-demo) 翻译输出分析工具。
以 `meta/init-interface-report.md` 为核心输入，提取接口并分析 Rust 测试覆盖关系。

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
| `interface` | 读取 `meta/init-interface-report.md`，列出全部对外接口符号（函数 + 变量） |
| `scan` | 扫描 Rust `#[test]`，分类 ST/DT，并尝试匹配接口符号 |
| `coverage` | 按接口符号输出 ST/DT 覆盖矩阵（Markdown） |

## 核心输入

| 输入 | 说明 |
|---|---|
| `meta/init-interface-report.md` | `c2rust-demo init` 生成的接口报告（唯一接口来源） |

## 推荐工作流

```bash
# 1. 列出接口符号
c2rust-tests-helper interface

# 2. 扫描 Rust 测试并匹配接口
c2rust-tests-helper scan --dir rust

# 3. 输出覆盖矩阵
c2rust-tests-helper coverage --dir rust
```

默认接口报告路径是 `meta/init-interface-report.md`，可通过 `--report` 覆盖。
