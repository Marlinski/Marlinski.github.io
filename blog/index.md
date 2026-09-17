---
layout: default
title: Blog
---

{%- comment -%}
Tags ranked by post count. Same zero-pad-and-sort trick as the homepage cloud,
since Liquid cannot sort a hash by the size of its values.
{%- endcomment -%}
{%- assign pairs = "" | split: "" -%}
{%- for t in site.tags -%}
  {%- capture pair %}{{ t[1].size | prepend: "00" | slice: -2, 2 }}|{{ t[0] }}{% endcapture -%}
  {%- assign one = pair | split: "~~" -%}
  {%- assign pairs = pairs | concat: one -%}
{%- endfor -%}
{%- assign pairs = pairs | sort | reverse -%}

<div class="sec">
  <span class="sec-mark"></span>
  <h2>writing</h2>
  <span class="sec-rule"></span>
  <span class="sec-meta" id="post-count">{{ site.posts.size }} posts</span>
</div>

<div class="cloud" id="filters">
  <span class="cloud-label">filter</span>
  <button class="chip is-on" data-tag="all" type="button">all</button>
  {%- for pair in pairs limit: 10 -%}
  {%- assign bits = pair | split: "|" -%}
  {%- assign n = bits[0] | times: 1 -%}
  {%- if n > 1 -%}
  <button class="chip" data-tag="{{ bits[1] }}" type="button">#{{ bits[1] }}<span class="chip-n">{{ n }}</span></button>
  {%- endif -%}
  {%- endfor -%}
</div>

<ul class="posts">
{% for post in site.posts %}
  <li data-tags="{{ post.tags | join: ' ' }}">
    <div class="post-line">
      <time>{{ post.date | date: "%Y-%m-%d" }}</time>
      <a href="{{ post.url | relative_url }}">{{ post.title }}</a>
      {% if post.tags.size > 0 %}
      <span class="post-tags">{% for tag in post.tags limit:2 %}<span class="tag">{{ tag }}</span>{% endfor %}</span>
      {% endif %}
    </div>
    {% if post.blurb %}<p class="post-blurb">{{ post.blurb }}</p>{% endif %}
  </li>
{% else %}
  <li class="empty">no posts yet.</li>
{% endfor %}
</ul>

<p class="no-match" hidden>nothing tagged that. <button class="linkish" type="button" data-tag="all">show all</button></p>

<script>
// Filter the list in place. Without JS every post is visible, which is the
// correct fallback — the filter is a convenience, not the only way in.
(function () {
  var items = Array.prototype.slice.call(document.querySelectorAll('.posts > li[data-tags]'));
  var chips = Array.prototype.slice.call(document.querySelectorAll('[data-tag]'));
  var count = document.getElementById('post-count');
  var empty = document.querySelector('.no-match');
  var total = items.length;

  function apply(tag) {
    var shown = 0;
    var last = null;
    items.forEach(function (li) {
      var tags = (' ' + li.getAttribute('data-tags') + ' ');
      var hit = tag === 'all' || tags.indexOf(' ' + tag + ' ') !== -1;
      li.hidden = !hit;
      li.classList.remove('last-shown');
      if (hit) { shown++; last = li; }
    });
    // :last-child still counts hidden siblings, so mark the last visible row
    // ourselves to keep the trailing rule off.
    if (last) last.classList.add('last-shown');
    chips.forEach(function (c) {
      c.classList.toggle('is-on', c.classList.contains('chip') && c.getAttribute('data-tag') === tag);
    });
    count.textContent = tag === 'all' ? total + ' posts' : shown + ' of ' + total;
    empty.hidden = shown !== 0;
    if (history.replaceState) {
      history.replaceState(null, '', tag === 'all' ? location.pathname : '#' + tag);
    }
  }

  chips.forEach(function (c) {
    c.addEventListener('click', function () { apply(c.getAttribute('data-tag')); });
  });

  // Every tag any post carries, not just the ones that earned a chip — the
  // homepage cloud counts project tags too, so it links here with tags that
  // are rare enough on posts to have no button of their own.
  var known = ['all'];
  items.forEach(function (li) {
    li.getAttribute('data-tags').split(' ').forEach(function (t) {
      if (t && known.indexOf(t) === -1) known.push(t);
    });
  });

  function fromHash() {
    var t = decodeURIComponent(location.hash.replace('#', ''));
    apply(known.indexOf(t) !== -1 ? t : 'all');
  }
  window.addEventListener('hashchange', fromHash);
  if (location.hash) fromHash();
})();
</script>
