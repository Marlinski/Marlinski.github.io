---
layout: default
---

I build things that run where you already are: a terminal, a browser tab, a phone with no signal.
Research engineer, working across AI agents, cryptography, blockchain and decentralized networks —
currently at OVHcloud.
{: .intro}

France → Singapore → US → Canada.
{: .intro-sub}

{% assign active = site.data.projects | where: "status", "active" %}
{% assign archived = site.data.projects | where: "status", "archived" %}
{% assign featured = site.data.projects | where: "featured", true %}

<div class="sec">
  <span class="sec-mark"></span>
  <h2>building now</h2>
  <span class="sec-rule"></span>
</div>

<ul class="feat">
{% for p in featured %}
  <li>
    <a class="feat-card" href="{{ p.url }}"{% if p.url contains '://' %} target="_blank"{% endif %}>
      <div class="feat-shot">
        {% if p.shot %}
        <img src="{{ p.shot | relative_url }}" alt="{{ p.alt }}" loading="lazy">
        {% elsif p.term %}
        <div class="feat-term">{% for line in p.term %}<span class="t-{{ line.kind }}">{{ line.text }}</span>
{% endfor %}</div>
        {% endif %}
      </div>
      <div class="feat-body">
        <span class="feat-name"><span class="dot"></span>{{ p.name }}</span>
        <span class="feat-desc">{{ p.blurb | default: p.desc }}</span>
        <span class="feat-tags">{{ p.tags | join: " #" | prepend: "#" }}</span>
      </div>
    </a>
  </li>
{% endfor %}
</ul>

<div class="sec">
  <span class="sec-mark"></span>
  <h2>what i keep coming back to</h2>
  <span class="sec-rule"></span>
</div>

<ul class="focus">
{% for f in site.data.focus %}
  <li>
    <span class="focus-title">{{ f.title }}</span>
    <span class="focus-desc">{{ f.desc }}<span class="focus-refs">{{ f.refs }}</span></span>
  </li>
{% endfor %}
</ul>

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

<div class="cloud">
  <span class="cloud-label">tags</span>
  {%- for pair in pairs limit: 12 -%}
  {%- assign bits = pair | split: "|" -%}
  <a class="chip" href="{{ '/blog' | relative_url }}#{{ bits[1] }}">#{{ bits[1] }}<span class="chip-n">{{ bits[0] | times: 1 }}</span></a>
  {%- endfor -%}
</div>

<div class="sec">
  <span class="sec-mark"></span>
  <h2>all projects</h2>
  <span class="sec-rule"></span>
  <span class="sec-meta">{{ active.size }} active · {{ archived.size }} archived</span>
</div>

<ul class="proj-list">
{% for p in active %}
  {%- assign linked = site.posts | where_exp: "x", "x.path contains p.post" | first -%}
  <li>
    <span class="dot"></span>
    <span class="proj-title"><a href="{{ p.url }}"{% if p.url contains '://' %} target="_blank"{% endif %}>{{ p.name }}</a>{% if p.gh %}<a href="{{ p.gh }}" class="proj-gh" target="_blank">[gh]</a>{% endif %}{% if linked %}<a href="{{ linked.url | relative_url }}" class="proj-gh">[post]</a>{% endif %}</span>
    <span class="proj-desc">{{ p.desc }}</span>
    <span class="proj-tags">{{ p.tags | join: " #" | prepend: "#" }}</span>
  </li>
{% endfor %}
</ul>

<div class="group-rule"><span>archive</span></div>

<ul class="proj-list archived">
{% for p in archived %}
  {%- assign linked = site.posts | where_exp: "x", "x.path contains p.post" | first -%}
  <li>
    <span class="dot"></span>
    <span class="proj-title"><a href="{{ p.url }}" target="_blank">{{ p.name }}</a>{% if linked %}<a href="{{ linked.url | relative_url }}" class="proj-gh">[post]</a>{% endif %}</span>
    <span class="proj-desc">{{ p.desc }}{% if p.years %} <span class="proj-years">{{ p.years }}</span>{% endif %}</span>
    <span class="proj-tags">{{ p.tags | join: " #" | prepend: "#" }}</span>
  </li>
{% endfor %}
</ul>

<div class="sec">
  <span class="sec-mark"></span>
  <h2>writing</h2>
  <span class="sec-rule"></span>
  <a class="sec-meta" href="{{ '/blog' | relative_url }}">all {{ site.posts.size }} →</a>
</div>

<ul class="sq-list posts-compact">
{% for post in site.posts limit:5 %}
  <li>
    <time>{{ post.date | date: "%Y-%m" }}</time>
    <a href="{{ post.url | relative_url }}">{{ post.title }}</a>
    <span class="post-tags">{% for tag in post.tags limit:2 %}<span class="tag">{{ tag }}</span>{% endfor %}</span>
  </li>
{% endfor %}
</ul>

<div class="sec">
  <span class="sec-mark"></span>
  <h2>elsewhere</h2>
  <span class="sec-rule"></span>
</div>

All projects on [GitHub](https://github.com/Marlinski). Slides and course material under [/public]({{ '/public' | relative_url }}).
