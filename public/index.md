---
layout: default
title: Public
---

{%- assign presentations = site.data.public | where: "kind", "presentation" -%}
{%- assign teachings = site.data.public | where: "kind", "teaching" -%}

## presentations

<ul class="sq-list">
{% for e in presentations %}
  <li>
    <details class="entry">
      <summary>
        <a href="{{ e.dir }}/{{ e.files[0].name }}" target="_blank">{{ e.title }}</a>{% if e.where %} — {{ e.where }}{% endif %}
        <span class="meta">{{ e.date }}</span>
      </summary>
      <ul class="entry-files">
      {%- for f in e.files %}
        <li><a href="{{ e.dir }}/{{ f.name }}" target="_blank">{{ f.name }}</a>{% if f.desc %} <span class="file-desc">{{ f.desc }}</span>{% endif %}</li>
      {%- endfor %}
      </ul>
    </details>
  </li>
{% endfor %}
</ul>

## teachings

<ul class="sq-list">
{% for e in teachings %}
  <li>
    <details class="entry">
      <summary>
        <a href="{{ e.dir }}/{{ e.files[0].name }}" target="_blank">{{ e.title }}</a>{% if e.where %} — {{ e.where }}{% endif %}
        <span class="meta">{{ e.date }}</span>
      </summary>
      <ul class="entry-files">
      {%- for f in e.files %}
        <li><a href="{{ e.dir }}/{{ f.name }}" target="_blank">{{ f.name }}</a>{% if f.desc %} <span class="file-desc">{{ f.desc }}</span>{% endif %}</li>
      {%- endfor %}
      </ul>
    </details>
  </li>
{% endfor %}
</ul>
