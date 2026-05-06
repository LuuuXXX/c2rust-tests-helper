# c2rust-tests-helper

一个用于管理 C 测试迁移到基于 Rust FFI 测试的极简 CLI 工具。

在将 C 项目的测试套件移植到 Rust（同时保持 C 实现不变）时，你需要：

1. **追踪** 哪些 C 测试已存在，哪些已完成移植。
2. **运行** 原始 C 测试和新的 Rust 测试，确保两者结果一致。
3. **报告** 随时间推移的迁移进度。

`c2rust-tests-helper` 通过一个 YAML 文件和三个子命令处理以上全部步骤。

---

## 安装

```bash
cargo install --path .
```

或在本地构建：

```bash
cargo build --release
# 二进制文件位于 ./target/release/c2rust-tests-helper
```

---

## 快速上手

### 1. 创建 / 定制 `helper.yml`

将项目中附带的 `helper.yml` 复制到你的项目根目录，并编辑 `config` 部分，指向你的 C 源码目录和测试命令：

```yaml
config:
  discovery:
    paths:
      - tests/c          # 扫描 C 测试文件的目录
    patterns:
      - regex: "void\\s+(test_\\w+)\\s*\\("   # 匹配 "void test_foo("
        framework: custom
  c_test_command: "make test"
  rust_test_command: "cargo test"
```

### 2. 发现 C 测试

```bash
c2rust-tests-helper collect --config helper.yml
```

该命令遍历配置的 `paths`，应用每条正则表达式，并将新发现的测试名称**合并**到 `helper.yml` 的 `tests:` 列表中。已有条目不会被覆盖，手动编辑的 `status`、`rust_tests` 和 `notes` 字段会被保留。

### 3. 运行两套测试并检查一致性

```bash
c2rust-tests-helper check --config helper.yml
```

该命令将：

* 验证清单文件（数据有误时给出明确错误）。
* 运行 `c_test_command` 和 `rust_test_command`。
* 打印每套测试的通过/失败摘要。
* 将结果写入结果文件（默认为 `helper-results.yml`），以便 `report` 无需重新运行测试即可显示上次结果。

### 4. 报告迁移进度

```bash
c2rust-tests-helper report --config helper.yml
```

打印内容：

* 按迁移状态分类的统计（已移植 / 待移植 / 已跳过 / 不适用）。
* 待移植条目的待办列表。
* 上次 C 和 Rust 测试的运行结果（如果 `helper-results.yml` 存在）。

---

## 文件说明

| 文件 | 用途 |
|------|------|
| `helper.yml` | 主清单**兼**配置文件——单一信息来源。 |
| `helper-results.yml` | 由 `check` 写入，由 `report` 读取。缓存上次运行结果，使 `report` 快速响应。 |

> **为什么要两个文件？** 运行测试套件可能很慢。将结果缓存到单独的文件中，可以让 `report` 即时显示结果而无需重新执行任何操作。结果文件有意独立存放，如需要可将其加入 `.gitignore` 忽略版本控制。

---

## 清单格式

`tests:` 中的每条记录跟踪一个 C 测试：

```yaml
tests:
  - name: test_add_positive        # C 测试函数名
    source_file: tests/c/test_math.c
    status: ported                 # pending | ported | skipped | not_applicable
    rust_tests:
      - test_add_positive_numbers  # 覆盖该 C 测试的 Rust 测试名
    notes: "通过公共 add() FFI 封装直接 1:1 移植。"
```

### 迁移状态说明

| 状态 | 含义 |
|------|------|
| `pending` | 尚未编写对应的 Rust 测试（新发现条目的默认值）。 |
| `ported` | 已编写覆盖该 C 测试的 Rust 测试。 |
| `skipped` | 有意跳过（例如，测试的是无法通过公共 FFI 访问的内部代码）。 |
| `not_applicable` | 不是真正的测试（辅助函数 / 初始化函数）。 |

---

## 自定义 C 测试框架

`discovery.patterns` 列表接受任意数量的正则表达式，第一个捕获组被提取为测试名称。你可以为任何自定义或自研的 C 测试框架添加条目：

```yaml
patterns:
  # Unity 框架
  - regex: "(?:^|\\s)TEST\\(\\s*(\\w+)\\s*\\)"
    framework: unity
  # 自研宏 MY_TEST(suite, name) — 捕获完整的 "suite_name"
  - regex: "MY_TEST\\(\\s*(\\w+)\\s*,\\s*(\\w+)\\s*\\)"
    framework: my_framework
```

> **多捕获组说明：** 仅使用**第一个**捕获组作为测试名称。如果你的框架使用两个组（套件名 + 测试名），可将它们合并到一个非捕获组中，或调整正则使第 1 组包含完整标识符。

---

## CLI 参考

```
c2rust-tests-helper <COMMAND> [OPTIONS]

子命令：
  collect   扫描 C 源文件并将新测试条目合并到清单中
  check     验证清单，运行 C 和 Rust 测试，写入结果，打印报告
  report    打印迁移摘要（以及可选的上次运行结果）

选项（所有子命令）：
  -c, --config <FILE>    清单/配置 YAML 文件路径 [默认值: helper.yml]

选项（仅 check）：
  --results <FILE>       覆盖结果输出文件路径

选项（仅 report）：
  --results <FILE>       从指定文件加载结果，而非使用默认文件
```

---

## 扩展工具

代码结构便于扩展：

| 模块 | 职责 |
|------|------|
| `src/manifest.rs` | 数据结构、YAML 读写、校验 |
| `src/collect.rs` | 发现逻辑（在此添加新的解析器） |
| `src/check.rs` | 命令执行与结果捕获 |
| `src/report.rs` | 报告输出（在此添加更丰富的格式化器） |
| `src/runner.rs` | Shell 命令运行器（跨平台） |
| `src/cli.rs` | CLI 定义（在此添加新子命令） |

