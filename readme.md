# Markdown to HTML parser

This commandline program parses Markdown files to HTML files but doesn't add any \<html\>, \<script\>, \<head\> or its inner or \<body\> tags. Current version can manage headers (\<h1\>-\<h6\>), links \<a\>, images \<img\>, paragraphs \<p\>, bold \<b\> and italic \<i\> .

## What is not supported in this version that you might need or want

- unordered lists
- ordered lists
- underscore
- strikethrough

All previous mentions will be supported in future versions. And in the more distant future, I will make this able to parse html back to markdown so I can edit my blog texts easily.

# Why this project?

I needed a markdown parser for my blog because writing markdown is a clear way to format text. I could have gone a different route, like just detecting new lines to parse into paragraphs but it wasn't enough. I also wanted something to hone my coding skills with, so this came into mind. I could have used ready programs but why use readily available programs when you can bang your head to the wall?

This code is propably very confusing to look at because there are hundreds of rows of match arms. I wanted to make it by using match arms because they are so powerful in rust.
