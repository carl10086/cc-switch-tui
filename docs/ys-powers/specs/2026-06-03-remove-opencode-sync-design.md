# Spec: 去掉 model 与 opencodeModelId 的自动同步

## Objective

当前 `InstanceDetailPage` 和 `InstanceForm` 在切换 model 时会自动同步 opencodeModelId，强制两者保持一致。用户希望 model 和 opencodeModelId 完全独立，由用户手动选择。

### 当前问题

- 切换 model → opencodeModelId 被自动覆盖，用户无法自由组合
- 同步逻辑基于 `template.models[].opencodeModelId`（旧数据源），与新的 `template.opencodeModels` 不一致

### 正确行为

- 切换 model **不**影响 opencodeModelId
- opencodeModelId 保持用户上次选择的值（或空）
- 创建实例时，opencodeModelId 默认为空（由后端/Claude Code 自行处理）

## Commands

```bash
cargo test          # 后端测试
make typecheck      # cd web && npx tsc --noEmit
cd web && npx vitest run  # 前端测试
```

## Project Structure

### 前端修改

| 文件 | 改动 |
|---|---|
| `web/src/routes/InstanceDetailPage.tsx` | 删除 useEffect 同步逻辑（第 47-62 行） |
| `web/src/components/InstanceForm.tsx` | 删除 useEffect 同步逻辑（第 68-74 行） |
| `web/src/routes/__tests__/InstanceDetailPage.test.tsx` | 删除或修改 "changing model syncs opencodeModelId" 测试 |

## Code Style

直接删除 useEffect 代码块，不留注释。不引入新抽象。

## Testing Strategy

- 现有测试：删除已失效的同步测试
- 不新增测试（行为变简单，无需额外覆盖）
- `npx vitest run` 全部 pass

## Boundaries

- **Always do**：跑测试再 commit；删除同步逻辑后检查是否有 orphaned imports/variables
- **Ask first**：无
- **Never do**：不引入新的同步逻辑；不改后端 API；不改 OpencodeModelSelect 组件

## Success Criteria

- [ ] InstanceDetailPage 切换 model 后 opencodeModelId 不变
- [ ] InstanceForm 切换 model 后 opencodeModelId 不变
- [ ] 前端测试全部 pass
- [ ] git log 显示 1 个 commit
