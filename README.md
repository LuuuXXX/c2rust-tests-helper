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
| `surface` | 打印 feature surface（已选文件、模块、符号） |
| `collect` | 扫描 C 测试并合并写入 `migration.yml` |
| `lint` | 对照 feature surface 校验 manifest 中的所有条目 |
| `report` | 显示迁移进度与覆盖/缺口摘要 |
| `check` | 执行 lint，再运行配置的测试套件，最后打印统一摘要 |

## 核心输入

| 输入 | 说明 |
|---|---|
| `migration.yml` | 核心 manifest：配置、发现规则、测试条目表 |
| `.c2rust/<feature>/...` | `c2rust-demo` 生成的 feature workspace（已选文件、模块、符号） |

## 推荐工作流

```bash
# 1. 查看生成的 feature surface。
c2rust-tests-helper surface --config migration.yml

# 2. 扫描 C 测试，写入 migration.yml。
c2rust-tests-helper collect --config migration.yml

# 3. 手动编辑 migration.yml，填写 module/symbols/rust_tests 映射。
$EDITOR migration.yml

# 4. 对照 feature surface 校验编辑内容。
c2rust-tests-helper lint --config migration.yml

# 5. 查看迁移进度与覆盖缺口。
c2rust-tests-helper report --config migration.yml

# 6. 执行 lint + 所有已配置的测试套件。
c2rust-tests-helper check --config migration.yml
```

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
  c: "make test"                              # 运行原始 C 测试
  rust: "cargo test"                          # 运行翻译后的 Rust 测试
  feature_rust: "cargo test --features default"  # 可选：feature 专项 Rust 测试

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

## 当前边界

**本工具已实现：**
- manifest 管理（`migration.yml` 读写、schema 校验）
- feature surface 校验（来自 `.c2rust/<feature>/...` 的已选文件、模块、符号）
- 覆盖率与缺口报告
- 测试执行编排（lint gate → 执行 `test_commands.*` → 统一摘要）

**本工具暂未实现：**
- 自动测试→模块/符号推断
- Rust AST 分析
- 历史结果缓存
- JSON/JUnit 报告输出
- 并行测试执行

## 当前状态

- **PR1** – 配置 schema、feature surface 加载、`surface` 子命令
- **PR2** – `collect`、`lint`、`report` 子命令；演进后的 `TestEntry` schema
- **PR3** – `check` 子命令：lint gate + 测试执行 + 统一摘要
