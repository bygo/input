# 中→英口语直译测评（50 条）— qwen3.5:2b

- **模型**：`qwen3.5:2b`（`think: false`，已常驻）
- **报告**：`report-literal-1788114925288.json`
- **对照**：同 prompt / 同 case

## 总分对比

| 模型 | ✓ 地道 | △ 勉强 | ✗ 不行 | ✓ 占比 |
|------|--------|--------|--------|--------|
| **2B** | 21 | 16 | 13 | **42%** |
| 4B | 33 | 10 | 7 | **66%** |
| 9B | 34 | 8 | 8 | **68%** |

**结论：2B 明显不如 4B，不是「差不多」。** 少用 `talk about`，但大量 **leave it 一刀切** 和 **义偏**。

## 2B 典型问题

1. **时间词糊掉**：过两天/改天 → until tomorrow  
2. **动作义翻反**：忙完再说 → Get busy first.；上车再说 → Get on the bus later.  
3. **习语跑偏**：你看着办 → Look after it.；怎么说呢 → what do you think?；差不多了 → about time；别提了 → Ugh.  
4. **递进义失败**：再说我忙 → I'm too busy to talk.（应为 Besides…）  
5. **直译回潮**：研究研究再说 → …then say something.

## 分条判定

| # | 原文 | 译文 | 判 |
|---|------|------|----|
| 1 | 我看看再说 | Let's take a look first. | ✓ |
| 2 | 明天再说 | Let's leave it until tomorrow. | ✓ |
| 3 | 再说吧 | Let's leave it for later. | ✓ |
| 4 | 回头再说 | Let's leave it for later. | ✓ |
| 5 | 有空再说 | Let's leave it until later. | △ 丢「有空」 |
| 6 | 到时候再说 | Let's leave it until later. | ✓ |
| 7 | 过两天再说 | Let's leave it until tomorrow. | ✗ 两天≠明天 |
| 8 | 以后再说 | Later. | ✓ |
| 9 | 改天再说 | Let's leave it until tomorrow. | △ |
| 10 | 下次再说 | Let's leave it for later. | △ 丢「下次」 |
| 11 | 等会儿再说 | Let's leave it until later. | ✓ |
| 12 | 忙完再说 | Get busy first. | ✗ 义反 |
| 13 | 先这样吧 | Let's leave it for now. | ✓ |
| 14 | 先放一放 | Put it aside for now. | ✓ |
| 15 | 先不说了 | Let's not talk about it yet. | △ |
| 16 | 算了吧 | Let's leave it. | △ 宜 Forget it. |
| 17 | 得了吧 | Get off my case. | △ |
| 18 | 随便你 | Whatever you want. | ✓ |
| 19 | 你看着办 | Look after it. | ✗ 宜 Up to you. |
| 20 | 怎么说呢 | Well, what do you think? | ✗ 宜 How should I put it? |
| 21 | 不好说 | It's hard to say. | ✓ |
| 22 | 看情况 | Check the situation. | △ 宜 Depends. |
| 23 | 走着瞧 | Let's see. | ✓ |
| 24 | 差不多了 | It's about time. | ✗ 义偏 |
| 25 | 差不多得了 | It's enough. | ✓ |
| 26 | 行吧 | Alright. | ✓ |
| 27 | 好吧 | Okay. | ✓ |
| 28 | 随缘吧 | Just let it be. | ✓ |
| 29 | 别提了 | Ugh. | ✗ 宜 Never mind. |
| 30 | 怎么搞的 | What's up? | △ |
| 31 | 搞什么呢 | What's up? | △ |
| 32 | 回头聊 | Let's talk later. | ✓ |
| 33 | 回头跟你说 | I'll tell you later. | ✓ |
| 34 | 有消息再说 | Just wait for news. | ✓ |
| 35 | 定了再说 | Let's leave it until later. | △ |
| 36 | 确认了再说 | Let's confirm later. | ✗ 宜 once confirmed |
| 37 | 商量再说 | Let's leave it for later. | ✗ 宜 … first |
| 38 | 考虑一下再说 | Think about it first. | ✓ |
| 39 | 研究研究再说 | Let's take a look and then say something. | ✗ |
| 40 | 看看再说定 | Let's leave it until later. | ✗ 丢「看看」 |
| 41 | 再说也不迟 | It's never too late. | △ |
| 42 | 再说我忙 | I'm too busy to talk. | ✗ 递进义错 |
| 43 | 再说我走了 | I'll leave. | ✗ |
| 44 | 吃完再说 | Let's leave it until later. | △ 丢「吃完」 |
| 45 | 回去再说 | Let's leave it for later. | △ 丢「回去」 |
| 46 | 上车再说 | Get on the bus later. | ✗ |
| 47 | 如何读书呢 | How do I read books? | ✓ |
| 48 | 怎么弄啊 | How do I get this done? | ✓ |
| 49 | 先这样，回头再说 | First, let's leave it for later. | △ 结构糊 |
| 50 | 我再想想，明天再说 | I'll think about it, and we'll talk tomorrow. | △ |

## 小结

- **2B ≠ 4B**：地道率 **42% vs 66%**，掉约 **24 个点**。  
- 2B 反而少 `talk about`，但用万能 **leave it / later** 糊弄，细节和习语更差。  
- **产品默认仍应 4B**；2B 只适合极低配兜底，不能当「差不多」的替代。
