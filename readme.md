# Markdown to HTML parser

This commandline program parses Markdown files to HTML files but doesn't add any \<html\>, \<script\>, \<head\> or its inner or \<body\> tags. Current version can manage headers (\<h1\>-\<h6\>), links \<a\>, images \<img\>, paragraphs \<p\>, bold \<b\> underscore \<u\> and italic \<i\>.

The parser reads the entire contents of the source file into a buffer and then processes the contents from the beginning, one byte at a time. It doesn't search anything from the source. Instead it decides what to do with each byte according to its inner state. Only drawback is, that each time the state rises higher, the program allocates memory for the new state, and each time the state falls back, the higher state memory is freed. This might be done differently in the future versions but for now, it is what it is.

## What is not supported in this version that you might need or want

- unordered lists
- ordered lists
- strikethrough

All previous mentions will be supported in future versions. And in the more distant future, I will make this able to parse html back to markdown so I can edit my blog texts easily.

## What will likely never be supported

- Full html page creation, because it is simply not the purpose of this project
- Page content list, because it is not hard to create links with markdown \[\]\(\) key

## What might be supported but I don't need

- Combination of italic, bold, underscore and strikethrough, because they are so effortless to write by hand if I ever need them

## Problems of this software

You can fill your memory by writing _*_*_*_*_*_*_*_*_*_*_*_*.., because the program allocates new memory slot for the new states and you are constantly calling it to allocate new state. Do that enough and the memory is full and the program crashes. Of course this means that you would need a large Markdown file full of this type of text. I am not fixing this bug, because it doesn't bother me.

# Why this project?

I needed a markdown parser for my blog because writing markdown is a clear way to format text. I could have gone a different route, like just detecting new lines to parse into paragraphs but it wasn't enough. I also wanted something to hone my coding skills with, so this came into mind. I could have used ready programs but why use readily available programs when you can bang your head to the wall?

This code is propably very confusing to look at because there are hundreds of rows of match arms. I wanted to make it by using match arms because they are so powerful in rust.
