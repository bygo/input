# 中→英口语直译测评（50 条）— qwen3.5:9b

- **模型**：`qwen3.5:9b`（`think: false`，已常驻）
- **报告**：`report-literal-1788114449983.json`
- **对照**：同 prompt / 同 case，上一轮 `qwen3.5:4b` 约 **66%** 地道

## 总分

| 档 | 条数 | 占比 |
|----|------|------|
| ✓ 地道 / 可接受 | 34 | **68%** |
| △ 勉强 / 偏弱 | 8 | 16% |
| ✗ 不地道 / 义偏 | 8 | 16% |

**结论：比 4B 略好一点，仍远未接近 100%。** 主要瓶颈仍是「X再说 → talk about」。

## 相对 4B 的变化

**明显变好**

| 原文 | 4B | 9B |
|------|----|----|
| 明天再说 | talk about it tomorrow ✗ | leave it until tomorrow ✓ |
| 走着瞧 | Watch this. ✗ | We'll see. ✓ |
| 怎么说呢 | I mean, △ | How should I put it? ✓ |
| 得了吧 | Nonsense. △ | Come on. ✓ |
| 吃完再说 | leave it until after we've eaten ✓ | Let's eat first. ✓（更口语） |

**仍然差 / 新差**

| 原文 | 9B 译文 | 问题 |
|------|---------|------|
| 回头再说 | Let's talk about it later. | ✗ talk about |
| 下次再说 | Let's talk about it next time. | ✗ |
| 等会儿再说 | Let's talk about it later. | ✗ |
| 忙完再说 | Let's talk about it later. | ✗ 丢「忙完」 |
| 有消息再说 | Let's talk about it later. | ✗ 丢「有消息」 |
| 商量再说 | Let's talk about it later. | ✗ 宜 … first |
| 回去再说 | Let's talk about it later. | ✗ 丢「回去」 |
| 先这样，回头再说 | …talk about it later. | ✗ |

## 分条判定

| # | 原文 | 译文 | 判 |
|---|------|------|----|
| 1 | 我看看再说 | I'll take a look and then decide. | ✓ |
| 2 | 明天再说 | Let's leave it until tomorrow. | ✓ |
| 3 | 再说吧 | Let's leave it. | ✓ |
| 4 | 回头再说 | Let's talk about it later. | ✗ |
| 5 | 有空再说 | Let's talk later. | △ |
| 6 | 到时候再说 | We'll see. | ✓ |
| 7 | 过两天再说 | Let's leave it for a couple of days. | ✓ |
| 8 | 以后再说 | Let's leave it for later. | ✓ |
| 9 | 改天再说 | Let's talk another time. | △ |
| 10 | 下次再说 | Let's talk about it next time. | ✗ |
| 11 | 等会儿再说 | Let's talk about it later. | ✗ |
| 12 | 忙完再说 | Let's talk about it later. | ✗ |
| 13 | 先这样吧 | Let's leave it at that. | ✓ |
| 14 | 先放一放 | Let's put that aside for now. | ✓ |
| 15 | 先不说了 | Let's leave it for now. | ✓ |
| 16 | 算了吧 | Forget it. | ✓ |
| 17 | 得了吧 | Come on. | ✓ |
| 18 | 随便你 | Whatever you want. | ✓ |
| 19 | 你看着办 | You decide. | ✓ |
| 20 | 怎么说呢 | How should I put it? | ✓ |
| 21 | 不好说 | Hard to say. | ✓ |
| 22 | 看情况 | Depends. | ✓ |
| 23 | 走着瞧 | We'll see. | ✓ |
| 24 | 差不多了 | We're about done. | ✓ |
| 25 | 差不多得了 | Enough already. | ✓ |
| 26 | 行吧 | Alright. | ✓ |
| 27 | 好吧 | Okay. | ✓ |
| 28 | 随缘吧 | Let it be. | ✓ |
| 29 | 别提了 | Forget it. | △ 宜 Never mind. |
| 30 | 怎么搞的 | What's going on? | ✓ |
| 31 | 搞什么呢 | What are you up to? | ✓ |
| 32 | 回头聊 | Let's talk later. | ✓ |
| 33 | 回头跟你说 | I'll tell you later. | ✓ |
| 34 | 有消息再说 | Let's talk about it later. | ✗ |
| 35 | 定了再说 | Let's leave it until later. | △ 宜 Once it's decided. |
| 36 | 确认了再说 | Let's wait until it's confirmed. | ✓ |
| 37 | 商量再说 | Let's talk about it later. | ✗ 宜 … first |
| 38 | 考虑一下再说 | Think about it first. | ✓ |
| 39 | 研究研究再说 | Let's think about it and then decide. | △ |
| 40 | 看看再说定 | Take a look and we'll decide later. | ✓ |
| 41 | 再说也不迟 | It's not too late to talk about it. | △ |
| 42 | 再说我忙 | Besides, I'm busy. | ✓ |
| 43 | 再说我走了 | Besides, I'm leaving. | △ |
| 44 | 吃完再说 | Let's eat first. | ✓ |
| 45 | 回去再说 | Let's talk about it later. | ✗ |
| 46 | 上车再说 | Let's talk once we're on the bus. | △ |
| 47 | 如何读书呢 | How do you read books? | ✓ |
| 48 | 怎么弄啊 | How do I do this? | ✓ |
| 49 | 先这样，回头再说 | Leave it like this for now, and we'll talk about it later. | ✗ |
| 50 | 我再想想，明天再说 | I'll think about it and talk tomorrow. | △ |

## 小结

- **9B 对若干习语更稳**（明天再说、走着瞧、怎么说呢、得了吧）。
- **「再说」搁置簇仍大量 talk about**，地道率只从 ~66% → **~68%**，性价比不高。
- 结合内存成本，**面向普通用户仍应以 4B 为主**；9B 换不来接近满分。
