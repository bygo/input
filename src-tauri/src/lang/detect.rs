use super::script::{is_arabic, is_cyrillic, is_han, is_hangul, is_kana, is_latin}; // use：导入；super：上一级模块；::：路径分隔；{}：一次导入多个名

fn letters(text: &str) -> Vec<char> { // fn：定义函数；&str：字符串切片借用；->：返回类型；Vec<char>：可变长字符向量
    text.chars().filter(|c| c.is_alphabetic()).collect() // .：方法调用；chars()：按 Unicode 标量值迭代；filter：保留满足条件的项；|c|：闭包参数；is_alphabetic()：是否字母；collect()：收集成 Vec
}

fn ratio(chars: &[char], pred: fn(char) -> bool) -> f32 { // &[char]：切片（借用一段连续数据）；fn(char)->bool：函数指针类型；f32：32 位浮点
    if chars.is_empty() { // if：条件分支；is_empty()：长度为 0 则 true
        return 0.0; // return：提前返回；0.0：f32 字面量
    }
    let n = chars.iter().copied().filter(|c| pred(*c)).count(); // let：不可变绑定；iter()：逐元素引用迭代；copied()：把 &char 拷成 char；pred(*c)：解引用后调用函数指针；count()：计数
    n as f32 / chars.len() as f32 // as：类型转换；len()：长度；最后表达式无分号 = 函数返回值
}

pub fn fits_target(text: &str, lang: &str) -> bool { // pub：对外可见；bool：布尔类型
    let t = text.trim(); // trim()：去掉首尾空白，返回 &str
    if t.is_empty() {
        return false; // false：布尔假值
    }
    let chars = letters(t); // 调用本文件函数；类型由右侧推断为 Vec<char>
    if chars.is_empty() {
        return true; // true：布尔真值
    }
    let han = ratio(&chars, is_han); // &chars：把 Vec 借用为切片；is_han：函数名当作 fn(char)->bool 传入
    let latin = ratio(&chars, is_latin);

    if lang.contains("中文") { // contains：子串是否出现；"中文"：&str 字面量
        if has_forbidden_latin_in_chinese(t) {
            return false;
        }
        return han >= 0.12; // >=：大于等于比较，结果为 bool
    }
    if lang.contains("英语") || lang.contains("英文") { // ||：逻辑或（短路求值）
        return han == 0.0 && latin >= 0.55; // ==：相等；&&：逻辑与（短路求值）
    }
    false // 走到这里则返回 false（无 return 的尾表达式）
}

fn has_forbidden_latin_in_chinese(text: &str) -> bool { // 无 pub：仅当前模块内可见
    let mut buf = String::new(); // mut：可变绑定；String：堆上可增长字符串；::new()：关联函数（类似静态方法）
    let check = |tok: &str| -> bool { // 闭包赋给变量；|参数|：闭包语法；可写参数类型与 -> 返回类型
        if tok.is_empty() || tok.len() <= 2 { // <=：小于等于；||：或
            return false;
        }
        let chars: Vec<char> = tok.chars().collect(); // : Vec<char>：显式标注类型；collect 需知道目标类型
        let all_upper = chars.iter().all(|c| c.is_ascii_uppercase()); // all：全部元素满足谓词才为 true；is_ascii_uppercase：ASCII 大写
        let all_lower = chars.iter().all(|c| c.is_ascii_lowercase()); // is_ascii_lowercase：ASCII 小写
        if all_upper && chars.len() <= 6 {
            return false;
        }
        if !all_lower { // !：逻辑非
            return false;
        }
        chars.len() >= 3 // 尾表达式：长度 >= 3 则为 true，否则 false
    };
    for c in text.chars() { // for ... in：遍历迭代器；c 类型为 char
        if c.is_ascii_alphabetic() { // is_ascii_alphabetic：是否 ASCII 字母 a-z/A-Z
            buf.push(c); // push：往 String 末尾追加一个 char
        } else if !buf.is_empty() { // else if：否则若
            if check(&buf) { // &buf：&String 可自动解引用成 &str（Deref 强制）
                return true;
            }
            buf.clear(); // clear：清空内容，保留已分配容量
        }
    }
    if !buf.is_empty() && check(&buf) { // 循环结束后处理尾部残留 token
        return true;
    }
    false
}

pub fn is_pure_english(text: &str) -> bool {
    let mut any_latin = false; // 可变标志，初始为 false
    for c in text.chars() {
        if c.is_ascii_alphabetic() {
            any_latin = true; // 赋值（仅 mut 变量可改）
            continue; // continue：跳过本次循环剩余部分，进入下一轮
        }
        if is_han(c) || is_kana(c) || is_hangul(c) || is_cyrillic(c) || is_arabic(c) { // 多个 || 串联
            return false;
        }
        if c.is_alphabetic() { // 其它非 ASCII 字母脚本
            return false;
        }
    }
    any_latin // 有过拉丁字母且未提前 return false → 返回该标志
}

pub fn route(text: &str, _native: &str, _target: &str) -> String { // _前缀：刻意未使用的参数，避免 unused 警告
    if is_pure_english(text) {
        "中文".into() // into()：From/Into 转换，这里 &str → String
    } else {
        "英语".into() // if-else 两支类型一致，整段作为返回值
    }
}

pub fn into_for_selection(text: &str) -> String {
    route(text, "中文", "英语") // 调用并直接作为返回值；字面量传给 &str 参数
}
