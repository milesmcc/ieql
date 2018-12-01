# IEQL Specification (v0.1.0)

This document specifies the format of IEQL queries. For an overview of IEQL, please see [README.md](README.md).

## Data Format

IEQL queries must be valid JSON objects. They must have the root keys `triggers`, `scope`, and `threshold` (all other root keys are optional). Note that the following is not valid JSON; instead, it is meant to provide a primer on the general structure of a IEQL query.

```json
{
  "triggers": ["<array of trigger objects>"],
  "scope": {
    "documents": "<RegEx to match URLs to consider, or `*`>",
    "content": "<the type of content to match>"
  },
  "threshold": {
    "considers": ["<array of trigger IDs and/or other compositions>"],
    "required": "<number of considered items required for a match>"
  }
}
```

#### Trigger Object

A **trigger object** must have two keys: `type`, `pattern`, and `id`. `type` defines the type of matching to perform, and may be one of `regex` (for RegEx matching) or `raw` (for literal content check). (These types are strings.) `pattern` (string) defines that which is actually being searched for (this would be, for example, a RegEx pattern). Finally, `id` is the unique ID assigned to the trigger object that will be referenced later in the `threshold` (string).

An example trigger object might look like the following:

```json
{
  "type": "regex",
  "pattern": "IEQL is ([ A-Za-z]+)(awesome|incredible)",
  "id": "awesomeQuery"
}
```

#### Scope

The **scope** defines the type of data that will be fed to the triggers. The **scope** must be a valid JSON object with two keys: `documents` and `content`.

**`documents`** (string) must be a valid RegEx pattern, and matches the URL of documents. For example, for a IEQL query to be only performed on `.net` or `.org` domains, `documents` would be set to `(https?:\/\/)?(www\.)?[a-z0-9-]+\.(net|org)(\.[a-z]{2,3})?`. It seems complicated, but that's the power of RegEx! To match all URLs, set `documents` to `.+`.

**`content`** (string) may be one of `raw`, `text`, or `headers`. (Additional values are possible; refer to your implementations' source code for more information about the available content types.) `raw` means that the triggers will be fed the raw HTTP response—headers, HTML, and all. `text` means that the trigger will be fed a cleaned version of the document with only its text. `headers` means that the triggers will be passed just the HTML headers (unmodified).

An example scope definition might be as follows:

```json
...
"scope": {
  "documents": ".+",
  "content": "text"
}
...
```

This scope would consider all Internet documents to be in scope, and would feed the preprocessed text to the triggers.

#### Threshold

The **threshold** defines what triggers are required in order for the document to be a match. The threshold object must have the root keys `considers`, `required`, and optionally `inverse`.

**`considers`** (array) lists the various triggers and/or other threshold objects that should be considered. Triggers are identified by their IDs (strings), while other thresholds are themselves valid threshold objects. In this way, a threshold object may itself contain other threshold objects. (See below for an example.)

**`required`** (integer) defines the minimum number of objects listed in `considers` that must evaluate to `true` in order for the threshold to be met (and the IEQL query to match). For an `OR`-like relationship between `considers`, `required` should be `1`. For an `AND`-like relationship, `required` should be the total number of objects in `considers`. If `required` is greater than the number of objects in `considers`, the threshold will never be met; conversely, if `required` is `0`, the threshold will _always_ be met.

**`inverse`** (boolean; optional; default is `false`) determines whether the result will be inversed. With the inclusion of `inverse`, _any boolean expression can be expressed in IEQL_.

If three triggers are available, `A`, `B`, and `C`, we can define a threshold to match whenever any match as follows:

```json
{
  "considers": ["A", "B", "C"],
  "required": 1
}
```

Alternatively, if we want _all_ triggers to be required for a match, we can write the following threshold:

```json
{
  "considers": ["A", "B", "C"],
  "required": 3
}
```

If we want _no_ triggers to match—i.e., a match is defined as whenever _no triggers_ are activated—we could define the following threshold:

```json
{
  "considers": ["A", "B", "C"],
  "required": 3,
  "inverse": true
}
```

Finally, if we want to define a threshold in which `A` _must_ match _and_ either `B` or `C` (or both) must match, we can write the following threshold:

```json
{
  "considers": [
    "A",
    {
      "considers": ["B", "C"],
      "required": 1
    }
  ],
  "required": 2
}
```

Threshold composition is very powerful!

## Philosophy

As you implement or compose IEQL queries, keep the following in mind:

- All the triggers are fed the same data; triggers may not have different scopes or preprocessing steps. If triggers require different scopes, they should not be part of the same IEQL query.
- All queries should be able to identify the specific part(s) of the document that caused them to trigger.
- You should _never_ allow non-trusted entities to create IEQL queries. If you are running IEQL queries on your own server, you should be careful which queries you run—after all, a malicious query could severely slow down your scanning infrastructure.
