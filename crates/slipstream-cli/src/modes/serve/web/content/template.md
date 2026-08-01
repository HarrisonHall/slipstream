# Slipstream {{feed}}

{% for entry in entries %}
## {{entry.title}}

By {{entry.author}}
{% if entry.content %}

{{entry.content|safe}}
{% endif %}
{% if entry.tags %}

- Tags
{% for tag in entry.tags %}
  - {{tag}}
{% endfor %}
{% endif %}

@{{entry.date}}

[Source]({{entry.source.url}})
{% if entry.comments.url %}
[Comments]({{entry.comments.url}})
{% endif %}
{% if entry.links|length %}
{% for link in entry.links %}

[{{link.title}}]({{link.url}})
{% endfor %}
{% endif %}

{% endfor %}
