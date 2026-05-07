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
| `status` | 查看迁移进度、覆盖率和未映射缺口 |
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

在 `c2rust-demo` 克隆目录旁（或任意位置）新建 `migration.yml`，文件中的路径均相对于该文件本身解析。

```yaml
version: 1

project:
  root: ../c2rust-demo      # c2rust-demo 项目根目录
  feature: default          # feature 名称

feature_source:
  kind: c2rust_feature
  root: ../c2rust-demo/.c2rust/default   # 生成的 feature workspace

test_commands:
  c: "make test"
  rust: "cargo test --features default"

discovery:
  paths:
    - tests/c           # 扫描目录，相对于 project.root
  extensions:
    - c
  patterns:
    - regex: 'void\s+(test_\w+)\s*\('
      framework: custom

tests:
  - c_test: test_add
    source_file: tests/c/math.c
    status: ported
    module: mod_math
    symbols:
      - add
    rust_tests:
      - test_add_ported
```

仓库中已附带可直接编辑的示例文件：[`migration.yml`](./migration.yml)。

`test_commands.rust` 支持完整的 Rust 测试命令，例如 `cargo test`、`cargo test --features default`，也可以是指定 package / 指定测试目标的命令。

## 当前边界

**本工具已实现：**
- manifest 管理（`migration.yml` 读写、schema 校验）
- feature surface 校验（来自 `.c2rust/<feature>/...` 的已选文件、模块、符号）
- 覆盖率与缺口报告
- 测试执行编排（manifest 校验 → 执行 `test_commands.c` / `test_commands.rust` → 统一摘要）

**本工具暂未实现：**
- 自动测试→模块/符号推断
- Rust AST 分析
- 历史结果缓存
- JSON/JUnit 报告输出
- 并行测试执行
