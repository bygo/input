# 中→英口语直译测评（50 条）— translategemma:4b

- **模型**：`translategemma:4b`（`think: false`，与 app 同款 select_prompt）
- **报告**：`report-literal-1788115787434.json`
- **设置**：型号列表已加入 `translategemma:4b`（UI：TG 4B）

## 总分对比

| 模型 | ✓ 地道 | △ 勉强 | ✗ 不行 | ✓ 占比 |
|------|--------|--------|--------|--------|
| 千问 2B | 21 | 16 | 13 | 42% |
| **TG 4B** | **22** | **12** | **16** | **44%** |
| 千问 4B | 33 | 10 | 7 | **66%** |
| 千问 9B | 34 | 8 | 8 | **68%** |

**结论：TranslateGemma-4B 在这套口语「再说」集上不如千问 4B/9B。**  
专精 MT 的「忠实 / discuss·talk」偏好，正好踩中搁置义失败模式。

## 主要失败

- **大片 talk about / discuss later**（明天/回头/有空/过两天/以后/改天/下次/等会儿/忙完/吃完/回去/上车/组合句…）
- **义偏**：你看着办 → What are you going to do now?；怎么搞的 → How do I do this?；定了再说 → Okay, let's do it.；确认了再说 → Confirmed.
- **尚可**：算了吧、得了吧、看情况、走着瞧、再说我忙、先这样吧 等短习语

## 分条判定

| # | 原文 | 译文 | 判 |
|---|------|------|----|
| 1 | 我看看再说 | Let me take a look. | ✓ |
| 2 | 明天再说 | Let's talk about it tomorrow. | ✗ |
| 3 | 再说吧 | Let's talk about it later. | ✗ |
| 4 | 回头再说 | We'll talk about it later. | ✗ |
| 5 | 有空再说 | Let's talk about it later. | ✗ |
| 6 | 到时候再说 | We'll see how it goes. | ✓ |
| 7 | 过两天再说 | We can talk about it later. | ✗ |
| 8 | 以后再说 | We can talk about it later. | ✗ |
| 9 | 改天再说 | We can talk about it another time. | ✗ |
| 10 | 下次再说 | We can discuss this later. | ✗ |
| 11 | 等会儿再说 | Let's talk about it later. | ✗ |
| 12 | 忙完再说 | We'll talk about it later. | ✗ |
| 13 | 先这样吧 | Let's just do it this way for now. | ✓ |
| 14 | 先放一放 | Let's hold on for a minute. | △ |
| 15 | 先不说了 | Let's move on. | △ |
| 16 | 算了吧 | Never mind. | ✓ |
| 17 | 得了吧 | Come on. | ✓ |
| 18 | 随便你 | Whatever. | ✓ |
| 19 | 你看着办 | What are you going to do now? | ✗ |
| 20 | 怎么说呢 | How should I say it? | △ |
| 21 | 不好说 | I don't know what to say. | △ |
| 22 | 看情况 | It depends. | ✓ |
| 23 | 走着瞧 | Let's see how it goes. | ✓ |
| 24 | 差不多了 | That's about it. | ✓ |
| 25 | 差不多得了 | That's enough. | ✓ |
| 26 | 行吧 | Okay then. | ✓ |
| 27 | 好吧 | Okay. | ✓ |
| 28 | 随缘吧 | Let it be. | ✓ |
| 29 | 别提了 | Let's not talk about it. | △ |
| 30 | 怎么搞的 | How do I do this? | ✗ |
| 31 | 搞什么呢 | What are you doing? | ✓ |
| 32 | 回头聊 | Let's catch up later. | ✓ |
| 33 | 回头跟你说 | I'll let you know later. | ✓ |
| 34 | 有消息再说 | I'll get back to you with more information. | △ |
| 35 | 定了再说 | Okay, let's do it. | ✗ |
| 36 | 确认了再说 | Confirmed. | ✗ |
| 37 | 商量再说 | Let's discuss it later. | △ 宜 first |
| 38 | 考虑一下再说 | Let's think about that later. | △ 宜 first |
| 39 | 研究研究再说 | Let me investigate that further. | ✓ |
| 40 | 看看再说定 | We'll see how it goes. | △ |
| 41 | 再说也不迟 | It's never too late. | △ |
| 42 | 再说我忙 | Besides, I'm busy. | ✓ |
| 43 | 再说我走了 | I'm heading out now. | △ |
| 44 | 吃完再说 | We'll talk about it after we've eaten. | ✗ |
| 45 | 回去再说 | We'll talk about it later. | ✗ |
| 46 | 上车再说 | Let's hop in and talk about it. | ✗ |
| 47 | 如何读书呢 | How should I read it? | △ |
| 48 | 怎么弄啊 | How do I do this? | ✓ |
| 49 | 先这样，回头再说 | Let's just do this first, and we can talk about it later. | ✗ |
| 50 | 我再想想，明天再说 | I'll think about it, and we can talk about it tomorrow. | ✗ |

## 小结

- 设置列表现为：千问 2B / 4B / 9B / **TG 4B**。  
- 同套口语 case 上 **TG4B（44%）≪ 千问4B（66%）**。  
- 专用 MT 更易出 `talk about`；**不能靠换成 TG4B 解决「再说」地道问题。**
