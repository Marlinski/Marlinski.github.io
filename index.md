---
layout: default
---

{%- comment -%}
Tag cloud: every tag from _data/projects.yml plus every post tag, counted and
ranked. Liquid has no group-by-count, so each tag is zero-padded into a
"06|ai" string, the list is sorted as text, then split back apart.
{%- endcomment -%}
{%- assign all_tags = "" | split: "" -%}
{%- for p in site.data.projects -%}{%- assign all_tags = all_tags | concat: p.tags -%}{%- endfor -%}
{%- for post in site.posts -%}{%- assign all_tags = all_tags | concat: post.tags -%}{%- endfor -%}
{%- assign uniq_tags = all_tags | uniq -%}
{%- assign pairs = "" | split: "" -%}
{%- for t in uniq_tags -%}
  {%- assign n = 0 -%}
  {%- for x in all_tags -%}{%- if x == t -%}{%- assign n = n | plus: 1 -%}{%- endif -%}{%- endfor -%}
  {%- capture pair %}{{ n | prepend: "00" | slice: -2, 2 }}|{{ t }}{% endcapture -%}
  {%- assign one = pair | split: "~~" -%}
  {%- assign pairs = pairs | concat: one -%}
{%- endfor -%}
{%- assign pairs = pairs | sort | reverse -%}

<div class="cloud cloud-top">
  {%- for pair in pairs limit: 14 -%}
  {%- assign bits = pair | split: "|" -%}
  <a class="chip" href="{{ '/blog' | relative_url }}#{{ bits[1] }}">#{{ bits[1] }}<span class="chip-n">{{ bits[0] | times: 1 }}</span></a>
  {%- endfor -%}
</div>

Research engineer, working across AI agents, cryptography, blockchain and decentralized networks —
currently at OVHcloud.

I am always interested in technology that pushes the boundaries of what networks, devices, and people
can do together — whether that's offline-first mobile apps, agentic systems, or cryptographic protocols.

<div class="sec">
  <span class="sec-mark"></span>
  <h2>writing</h2>
  <span class="sec-rule"></span>
  <a class="sec-meta" href="{{ '/blog' | relative_url }}">{% if site.posts.size > 6 %}all {{ site.posts.size }} →{% else %}blog →{% endif %}</a>
</div>

<ul class="sq-list posts-compact">
{% for post in site.posts limit:6 %}
  <li>
    <time>{{ post.date | date: "%Y-%m" }}</time>
    <a href="{{ post.url | relative_url }}">{{ post.title }}</a>
    <span class="post-tags">{% for tag in post.tags limit:2 %}<span class="tag">{{ tag }}</span>{% endfor %}</span>
  </li>
{% endfor %}
</ul>

{%- assign shelved = site.data.projects | where: "status", "archived" -%}
{%- assign moving = site.data.projects.size | minus: shelved.size -%}

<div class="sec">
  <span class="sec-mark"></span>
  <h2>projects</h2>
  <span class="sec-rule"></span>
  <span class="sec-meta">{{ moving }} of {{ site.data.projects.size }} still moving</span>
</div>

<ul class="feat">
{% for p in site.data.projects %}
  {%- assign linked = site.posts | where_exp: "x", "x.path contains p.post" | first -%}
  {%- comment -%}No `url` means the write-up is the main link, and [gh] carries the code.{%- endcomment -%}
  {%- if p.url -%}{%- assign main = p.url -%}{%- else -%}{%- assign main = linked.url | relative_url -%}{%- endif -%}
  <li class="{{ p.status }}">
    <a class="feat-shot" href="{{ main }}"{% if main contains '://' %} target="_blank"{% endif %} tabindex="-1" aria-hidden="true">
      {%- if p.shot %}<img src="{{ p.shot | relative_url }}" alt="" loading="lazy">
      {%- elsif p.term %}<span class="feat-term">{% for line in p.term %}<span class="t-{{ line.kind }}">{{ line.text }}</span>
{% endfor %}</span>{% endif %}
    </a>
    <div class="feat-body">
      <span class="feat-name">
        {% if p.status %}<span class="dot"></span>{% endif %}
        <a href="{{ main }}"{% if main contains '://' %} target="_blank"{% endif %}>{{ p.name }}</a>
        {%- if p.years %}<span class="proj-years">{{ p.years }}</span>{% endif -%}
      </span>
      <span class="feat-desc">{{ p.blurb }}</span>
      <span class="feat-foot">
        <span class="feat-tags">{{ p.tags | join: " #" | prepend: "#" }}</span>
        <span class="feat-links">{% if p.gh %}<a href="{{ p.gh }}" target="_blank">[gh]</a>{% endif %}{% if linked and p.url %}<a href="{{ linked.url | relative_url }}">[post]</a>{% endif %}</span>
      </span>
    </div>
  </li>
{% endfor %}
</ul>

<div class="sec">
  <span class="sec-mark"></span>
  <h2>elsewhere</h2>
  <span class="sec-rule"></span>
</div>

All projects on [GitHub](https://github.com/Marlinski). Slides and course material under [/public]({{ '/public' | relative_url }}).
