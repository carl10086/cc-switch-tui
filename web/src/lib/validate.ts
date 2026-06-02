/// 前端表单 schema（Zod）。与 Rust 端 validate_alias 规则镜像：
/// - alias: 小写字母 / 数字 / `-` / `_`，1-32 字符
/// - apiKey: 非空
/// - 字段错误通过 zod issues 路径定位（与 Rust ApiError.field 对齐）

import { z } from 'zod';

export const instanceSchema = z.object({
  templateId: z.string().min(1, 'Template is required'),
  alias: z
    .string()
    .min(1, 'Alias is required')
    .max(32, 'Alias must be 32 characters or less')
    .regex(
      /^[a-z0-9_-]+$/,
      'Alias can only contain lowercase letters, digits, "-" and "_"',
    ),
  modelId: z.string().min(1, 'Model is required'),
  apiKey: z.string().min(1, 'API key is required'),
  opencodeModelId: z.string().optional(),
  kvCacheEnabled: z.boolean().default(false),
  contextWindowEnabled: z.boolean().default(false),
});

export type InstanceFormValues = z.infer<typeof instanceSchema>;
