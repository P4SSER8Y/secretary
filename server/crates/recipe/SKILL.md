# 菜谱编写指南

菜谱文件存放在 `data/recipes/` 目录下，使用 **Markdown + YAML 前置元数据（Frontmatter）** 格式。服务启动时自动扫描并建立索引。

## 文件命名

- 文件名即菜谱名称（唯一标识符）：`红烧肉.md`、`糖醋排骨.md`
- 文件名必须唯一，不可重复
- 支持中文字符，可直接使用菜名作为文件名
- 仅扫描 `.md` 扩展名的文件
- `data/recipes/saved/` 子目录用于存放已保存的菜单，不会被扫描为菜谱

## 封面图

有三种方式指定封面图（按优先级）：

1. **在 frontmatter 中显式指定** `cover_image`（推荐）— 路径相对于该 md 文件所在目录
2. **自动探测**（frontmatter 未指定时）：
   - 与菜谱文件同名的图片（如 `红烧肉.jpg`）— `.jpg` / `.png` / `.webp` / `.jpeg`
   - `<菜名>/cover.jpg` — 放在以菜名命名的子目录中

示例：
- `cover_image: "hongshao-rou.jpg"` — 同目录下的图片
- `cover_image: "hongshao-rou/cover.jpg"` — 子目录中的图片
- 不写 `cover_image` — 服务按上述自动探测规则查找

## YAML Frontmatter 字段

菜谱文件必须以 `---` 开头和结尾的 YAML frontmatter，之后是 Markdown 正文。

### 必填字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | string | 唯一标识符（即菜名），如"红烧肉"。文件名应与之一致。 |
| `category` | string | 分类，用于 UI 分类 tab 筛选。建议用英文：`meat`、`vegetable`、`seafood`、`soup`、`staple`、`snack`、`cold-dish` 等 |
| `prep_time` | string | 准备时间，如 `"15m"` |
| `cook_time` | string | 烹饪时间，如 `"45m"` |
| `servings` | integer | 菜谱基准份数（食材用量以此为准），如 `4` |
| `difficulty` | string | 难度：`easy`、`medium`、`hard` |

### 可选字段

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `cover_image` | string | 自动探测 | 封面图文件名，路径相对于该 md 文件。如 `"hongshao-rou.jpg"` |
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
| `sub_ingredients` | array | 否 | 嵌套子食材列表。用于组合调料（如"浓盐葱姜水"包含盐、葱、姜、水），归类为一组显示 |

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

  # 组合食材 — 子食材归类为一组显示，各自独立参与换算
  - name: "浓盐葱姜水"
    sub_ingredients:
      - name: "盐"
        amount: 1.0
        unit: "勺"
      - name: "葱"
        hint: "适量"
      - name: "姜"
        hint: "适量"
      - name: "水"
        amount: 100.0
        unit: "ml"
```

### 组合食材（sub_ingredients）

当某些食材是组合调料（如"浓盐葱姜水"、"五香粉"等预制混合物），可使用 `sub_ingredients` 字段将子食材归类为一组：

- 父级 `name` 作为分组标签显示，可附带 `hint` 说明
- 子食材各自独立参与份量换算和汇总合并
- 同名组合食材在汇总时，同名同单位的子食材自动合并
- UI 中以折叠分组形式展示，不与其他食材混排

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
name: "红烧肉"
category: "meat"
prep_time: "15m"
cook_time: "45m"
servings: 4
difficulty: "medium"
cover_image: "hongshao-rou.jpg"
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

## 在线编辑

Web 端和 API 均支持菜谱的创建和修改。

### Web 编辑

- **修改已有菜谱**：点击菜谱卡片 → 详情弹窗底部「✏️ 编辑此菜谱」→ Markdown 编辑器
- **新建菜谱**：导航栏 🍽️ → 左侧抽屉「📝 新建菜谱」→ 输入菜名 → 编辑器
- **封面图**：详情弹窗或编辑器工具栏「🖼️ 封面」按钮 → 上传/更换封面图
- 编辑器基于 md-editor-v3，左侧编辑 / 右侧预览

### API 接口

| Method | Path | 说明 |
|---|---|---|
| `GET` | `/recipes/<name>/raw` | 获取原始 markdown 文本 |
| `PUT` | `/recipes/<name>/raw` | 保存菜谱（推 S3 → 同步回本地 → 重建索引） |
| `POST` | `/recipes/<name>/image` | 上传封面图（支持 jpg/png/webp） |

保存流程：解析验证 frontmatter → 推送文件到 S3 → 从 S3 同步回本地 + 重建索引。S3 不可用时保存失败（保护数据一致性）。

AI agent 可用 `PUT /recipes/<name>/raw` 直接上传菜谱，body 为完整 markdown，`Content-Type: text/plain`。

### 手动同步

- **热重载**（仅本地扫描）：`POST /api/recipes/reload`
- **S3 同步**：`POST /api/recipes/sync` 或左侧抽屉「🔄 同步」按钮

## 注意事项

- `name` 必须全局唯一，建议与文件名一致
- `servings` 是整数，代表基准份数
- 食材同名同单位才会在汇总时合并（如"生抽:tbsp"和"生抽:ml"视为不同）
- 定性食材（无 `amount`）不做换算也不合并
- `adjustable: false` 的菜谱在点菜界面不显示 +/- 按钮，但仍在食材清单中按默认份量计算
- 菜单保存在 `data/recipes/saved/` 目录，不会被索引为菜谱
- 保存菜谱时 frontmatter 会被解析验证，格式错误会返回失败提示
