# c2rust-tests-helper

[`c2rust-demo`](https://github.com/LuuuXXX/c2rust-demo) feature workspace 的测试迁移工作流工具。
以 `migration.yml` 和 `.c2rust/<feature>/...` 作为输入，管理、校验并执行 C 测试到 Rust 的迁移过程。

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
| `inspect` | 从迁移配置中查看 feature surface（特性表面）信息 |
| `discover` | 发现 C 测试并合并到 migration manifest（迁移清单） |
| `validate` | 基于 feature surface（特性表面）校验 migration manifest（迁移清单） |
| `status` | 查看迁移进度、覆盖率、未映射缺口，以及仍需用户补充的条目 |
| `verify` | 校验 manifest（迁移清单）并运行已配置测试套件 |

兼容命令（旧命令）仍然可用：`surface`、`collect`、`lint`、`report`、`check`。

## 核心输入

| 输入 | 说明 |
|---|---|
| `migration.yml` | 核心 manifest：配置、发现规则、测试条目表 |
| `.c2rust/<feature>/...` | `c2rust-demo` 生成的 feature workspace（已选文件、模块、符号） |

## 推荐工作流

```bash
# 1. 发现 C 测试，更新 migration.yml
c2rust-tests-helper discover

# 2. 编辑映射关系
$EDITOR migration.yml

# 3. 校验映射并运行配置的测试
c2rust-tests-helper verify

# 可选：查看迁移进度、覆盖率和缺口
c2rust-tests-helper status
```

默认配置文件就是 `migration.yml`，所以普通情况下不需要反复传 `--config migration.yml`。

`verify` 默认会先执行校验流程；`validate` 主要用于你只想单独做 manifest 校验、快速排查映射问题的场景。`inspect` 也属于高级/排查入口。

## 配置示例

推荐直接在被迁移项目根目录执行 helper，并把 `migration.yml` 放在该目录下。这样 `project.root` 和 `feature_source.root` 都可以省略；工具会默认使用当前 `migration.yml` 所在目录作为项目根目录，并将 feature workspace 解析为 `.c2rust/<feature>/`。

```yaml
version: 1

project:
  feature: default          # 必填：feature 名称

# 可选；如果省略，`verify` 会默认在 `.c2rust/<feature>/rust` 下执行 `cargo test`
test_commands:
  c: "make test"

discovery:
  paths:
    - tests/c           # 扫描目录，相对于 project.root（默认就是 migration.yml 所在目录）
  extensions:
    - c
  patterns:
    - regex: 'void\s+(test_\w+)\s*\('
      framework: custom

# Generated and updated by `c2rust-tests-helper discover`.
# Users normally only edit `status` / `rust_tests` / `contract` / `notes`.
tests: []
```

仓库中已附带可直接编辑的示例文件：[`migration.yml`](./migration.yml)。

`discover`（旧别名 `collect`）会优先读取 `.c2rust/<feature>/meta/selected_files.json` 与 `rust/src/mod_*/`，自动补齐缺失的 `selected_file`、`module`、`symbols` 等字段；通常你只需要补充 `status`、`rust_tests`、`contract`、`notes` 这类无法自动判断的信息。`status` 也会额外输出 `Need User Action` 区块，帮助你快速定位还需要手工确认或补写的测试条目。因此，示例里的 `tests: []` 才是推荐起点：先 `discover`，再补你真正需要维护的状态字段。

`test_commands.rust` 支持完整的 Rust 测试命令，例如 `cargo test`、`cargo test --features default`，也可以是指定 package / 指定测试目标的命令；如果省略该字段，`verify` 会默认在 `<feature_root>/rust` 下执行 `cargo test`，所以多数场景下不需要显式配置它。

## 当前边界

**本工具已实现：**
- manifest 管理（`migration.yml` 读写、schema 校验）
- feature surface 校验（来自 `.c2rust/<feature>/...` 的已选文件、模块、符号）
- 覆盖率与缺口报告
- 测试执行编排（manifest 校验 → 执行 `test_commands.c` / `test_commands.rust` → 统一摘要）

**本工具暂未实现：**
- Rust AST 分析
- 历史结果缓存
- JSON/JUnit 报告输出
- 并行测试执行
