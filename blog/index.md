---
layout: default
title: Blog
---

## blog

<ul class="posts">
{% for post in site.posts %}
  <li>
    <time>{{ post.date | date: "%Y-%m-%d" }}</time>
    <a href="{{ post.url | relative_url }}">{{ post.title }}</a>
    {% if post.tags.size > 0 %}
    <span class="post-tags">{% for tag in post.tags %}<span class="tag">{{ tag }}</span>{% endfor %}</span>
    {% endif %}
  </li>
{% else %}
  <li class="empty">no posts yet.</li>
{% endfor %}
</ul>
