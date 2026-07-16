# 菜谱编写指南

菜谱文件存放在 `data/recipes/` 目录下，使用 **Markdown + YAML 前置元数据（Frontmatter）** 格式。服务启动时自动扫描并建立索引。

## 文件命名

- 文件名即菜谱 ID，使用小写字母 + 连字符：`hongshao-rou.md`、`tangcu-paigu.md`
- 文件名必须唯一，不可重复
- 仅扫描 `.md` 扩展名的文件
- `data/recipes/saved/` 子目录用于存放已保存的菜单，不会被扫描为菜谱

## 封面图

封面图放在 `data/recipes/` 目录下，支持以下两种方式（按优先级查找）：

1. `<id>.jpg` / `.png` / `.webp` / `.jpeg` — 与菜谱文件同名
2. `<id>/cover.jpg` / `.png` / `.webp` / `.jpeg` — 放在以菜谱 ID 命名的子目录中

示例：`hongshao-rou.jpg` 或 `hongshao-rou/cover.jpg`

## YAML Frontmatter 字段

菜谱文件必须以 `---` 开头和结尾的 YAML frontmatter，之后是 Markdown 正文。

### 必填字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | string | 唯一标识符，与文件名一致 |
| `name` | string | 菜名，如"红烧肉" |
| `category` | string | 分类，用于 UI 分类 tab 筛选。建议用英文：`meat`、`vegetable`、`seafood`、`soup`、`staple`、`snack`、`cold-dish` 等 |
| `prep_time` | string | 准备时间，如 `"15m"` |
| `cook_time` | string | 烹饪时间，如 `"45m"` |
| `servings` | integer | 菜谱基准份数（食材用量以此为准），如 `4` |
| `difficulty` | string | 难度：`easy`、`medium`、`hard` |

### 可选字段

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `adjustable` | bool | `true` | 是否允许在点菜时调整份量。设为 `false` 则禁用 +/- 按钮 |
| `tags` | string[] | `[]` | 标签，如 `["家常", "猪肉", "红烧"]` |
| `ingredients` | array | `[]` | 食材列表，见下方详细说明 |

### 食材（Ingredient）字段

每个食材项包含：

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | string | **是** | 食材名称，如"五花肉" |
| `amount` | float | 否 | 数量，如 `500.0`。不填表示不可计量（见下方定性食材） |
| `unit` | string | 否 | 单位，如 `"g"`、`"ml"`、`"tbsp"`、`"slice"`、`"个"` |
| `hint` | string | 否 | 定性描述，如 `"适量"`、`"少许"`。与 `amount` 互斥 |
| `optional` | bool | 否 | 是否为可选食材，默认 `false`。可选食材在汇总列表中排在最后 |

### 定量食材 vs 定性食材

- **定量食材**：填写 `amount` + `unit`，点菜时按 `amount × portions / servings` 等比换算，同名同单位自动合并求和
- **定性食材**：不填 `amount`，填 `hint`（如"适量"、"少许"、"依个人口味"），不参与数值换算，不做合并

```yaml
ingredients:
  # 定量 — 参与换算
  - name: "五花肉"
    amount: 500.0
    unit: "g"
  - name: "生抽"
    amount: 2.0
    unit: "tbsp"

  # 定性 — 不参与换算
  - name: "盐"
    hint: "适量"
  - name: "料酒"
    hint: "少许"
    optional: true
```

## 食材换算说明

点菜时，每道菜可以调整份量（默认份量 = 菜谱的 `servings`），实际用量按公式计算：

```
实际用量 = amount × portions / servings
```

例如红烧肉 `servings: 4`、五花肉 `amount: 500.0`，当点菜时将该道菜调为 2 份：
- 实际五花肉用量 = `500.0 × 2 / 4 = 250.0g`

汇总食材清单中，同名同单位的食材自动合并求和，并标注 `used_in` 来源。

## Markdown 正文

第二个 `---` 之后的内容为 Markdown 正文，即做菜模式的步骤文档。支持标准 Markdown 语法：

```markdown
## 准备工作

五花肉洗净，姜切片备用。

## 步骤

1. **焯水**（10m）：五花肉切 3cm 方块，冷水下锅，水开后撇去浮沫，捞出沥干。
   > 水要完全没过肉，焯水时间不宜过长。

2. **炒糖色**（3m）：锅中放少许油，加入冰糖，小火慢炒至融化。
```

**建议**：
- 用 `##` 二级标题分段（准备工作 / 步骤 / 小贴士）
- 步骤用有序列表 `1.`，每步可注明预计耗时
- 注意事项用 `>` 引用块
- 可在正文中嵌入图片链接

## 完整示例

```markdown
---
id: "hongshao-rou"
name: "红烧肉"
category: "meat"
prep_time: "15m"
cook_time: "45m"
servings: 4
difficulty: "medium"
adjustable: true
tags:
  - 家常
  - 猪肉
  - 红烧
ingredients:
  - name: "五花肉"
    amount: 500.0
    unit: "g"
  - name: "生抽"
    amount: 2.0
    unit: "tbsp"
  - name: "老抽"
    amount: 1.0
    unit: "tbsp"
  - name: "冰糖"
    amount: 20.0
    unit: "g"
  - name: "姜"
    amount: 3.0
    unit: "slice"
  - name: "盐"
    hint: "适量"
  - name: "料酒"
    hint: "少许"
---

## 准备工作

五花肉洗净，姜切片备用。

## 步骤

1. **焯水**（10m）：五花肉切 3cm 方块，冷水下锅，水开后撇去浮沫，捞出沥干。
   > 水要完全没过肉，焯水时间不宜过长。

2. **炒糖色**（3m）：锅中放少许油，加入冰糖，小火慢炒至冰糖融化变成焦糖色。
   > 火候要小，避免糖烧焦发苦。

3. **翻炒上色**（2m）：放入焯好的五花肉，快速翻炒使每块肉均匀裹上糖色。

4. **炖煮**（40m）：加入生抽、老抽、姜片，倒入开水没过肉面，大火烧开后转小火加盖炖 40 分钟。

5. **收汁**（5m）：开盖转大火收汁，至汤汁浓稠包裹肉块即可出锅。
```

## 添加菜谱后

菜谱文件放入 `data/recipes/` 后，有两种方式使其生效：

1. **重启服务** — 服务启动时自动扫描
2. **热重载** — `POST /recipe/api/recipes/reload`，无需重启

## 注意事项

- `id` 必须全局唯一，建议与文件名一致
- `servings` 是整数，代表基准份数
- 食材同名同单位才会在汇总时合并（如"生抽:tbsp"和"生抽:ml"视为不同）
- 定性食材（无 `amount`）不做换算也不合并
- `adjustable: false` 的菜谱在点菜界面不显示 +/- 按钮，但仍在食材清单中按默认份量计算
- 菜单保存在 `data/recipes/saved/` 目录，不会被索引为菜谱
