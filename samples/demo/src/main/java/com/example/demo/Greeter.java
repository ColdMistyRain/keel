package com.example.demo;

/**
 * 一个简单的问候类：Keel 迁移 spike 的编辑内容示例。
 */
public final class Greeter {
    private final String name;

    public Greeter(String name) {
        this.name = name;
    }

    public String greet() {
        return "你好，" + name + "！这是 Keel 的 nsh spike。";
    }
}
