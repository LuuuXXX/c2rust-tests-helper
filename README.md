# c2rust-tests-helper

[`c2rust-demo`](https://github.com/LuuuXXX/c2rust-demo) 翻译辅助工具。
用于发现 C 侧测试函数，并将其与 `c2rust-demo` 生成的接口报告中的 interface symbols 建立候选映射。

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
| `discover` | 递归扫描 C 源文件，发现 `test_xxx` 测试函数，输出 C 测试清单 |
| `map` | 将发现的 C 测试函数与 interface symbols 建立候选映射，输出 Markdown 报告 |

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

`discover` 和 `map` 会在打印 stdout 的同时写入文件：

| 命令 | 默认输出文件 |
|---|---|
| `discover` | `c-test-discovery-report.md`（当前目录） |
| `map` | `<report目录>/c-test-map-report.md` |

可通过 `--output <path>` 覆盖默认输出路径。

## 推荐工作流

```bash
# 运行 c2rust-demo init 或 merge 后，在 feature 根目录执行：

# 1. 查看接口清单
c2rust-tests-helper interface

# 2. 扫描 C 测试目录，输出 c-test-discovery-report.md
c2rust-tests-helper discover --dir tests/c

# 3. 生成 C 测试 → interface 候选映射，输出 meta/c-test-map-report.md
c2rust-tests-helper map --dir tests/c
```

## 参数说明

### `interface`

- `-r, --report <path>`：接口报告路径（可选，不传时自动 fallback）

### `discover`

- `-d, --dir <path>`：C 源文件扫描根目录（默认 `.`）
- `-o, --output <path>`：发现报告输出路径（可选）

### `map`

- `-r, --report <path>`：接口报告路径（可选，不传时自动 fallback）
- `-d, --dir <path>`：C 源文件扫描根目录（默认 `.`）
- `-o, --output <path>`：候选映射报告输出路径（可选）

## C 测试发现规则

`discover` 和 `map` 递归扫描指定目录下的 `.c` 文件，使用以下启发式规则识别测试函数：

- 行首以 `void` 或 `int` 开头的函数定义
- 函数名以 `test_` 前缀开始
- 示例：`void test_add(void)`, `int test_counter_inc(void)`

## 候选映射说明

`map` 命令基于符号名词边界匹配（`\bsymbol_name\b`）在 C 测试源码中查找可能涉及的 interface symbols，
输出结果为**候选列表**（candidate symbols），不代表严格的覆盖结论。

## meta 目录产物全览

```text
meta/
├── init-interface-report.md      # c2rust-demo init 生成（可选）
├── merge-interface-report.md     # c2rust-demo merge 生成（可选）
└── c-test-map-report.md          # c2rust-tests-helper map 生成
```

## 示例

[`examples/cjson/`](examples/cjson/README.md) — 以 cJSON 为例，展示完整工作流：
c2rust-demo 生成接口报告 → `discover` 扫描 C 测试 → `map` 生成候选映射报告。

