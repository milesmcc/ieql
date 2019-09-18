# IEQL Specification (v0.1.0)

This document specifies the format of IEQL queries. For an overview of IEQL, please see [README.md](README.md).

## Data Format

IEQL queries must be valid RON objects. They must have the root keys `triggers`, `scope`, `threshold`, and `response` (all other root keys are optional). Note that the following is not valid RON; instead, it is meant to provide a primer on the general structure of a IEQL query.

```ron
Query (
  triggers: <array of trigger objects>,
  scope: (
    documents: <pattern object>,
    content: <the type of content to match>,
  ),
  threshold: (
    considers: <array of trigger IDs and/or other compositions>,
    required: <number of considered items required for a match>,
  ),
  response: (
    type: <`full` or `partial`>,
    include: <array of items to include>,
  ),
)
```

> **Why RON?**
> 
> We use RON for IEQL queries because it is type-explicit and verbose. Remember — you shouldn't be writing IEQL by hand! Instead, you should be using a tool to generate IEQL queries for you.

#### Pattern Object

**Pattern objects** are used for RegEx-like pattern matching throughout IEQL. They must have two keys: `content` and `kind`. `content` is the content to match (either a RegEx query or raw text), and `kind` defines whether the parser should treat `content` as RegEx or as raw text. (`kind` may be one of `Raw` or `RegEx`.)

An example pattern object is the following:

```ron
Pattern (
  content: "something.+",
  kind: RegEx,
)
```

#### Trigger Object

A **trigger object** must have two keys: `pattern` and `id`. `pattern` must be a valid pattern object. `id` is the unique ID assigned to the trigger object that will be referenced later in the `threshold` (string).

An example trigger object might look like the following:

```ron
Trigger (
  pattern: (
    content: "IEQL is ([ A-Za-z]+)(awesome|incredible)",
    kind: RegEx
  ),
  id: "awesomeQuery"
}
```

#### Scope

The **scope** defines the type of data that will be fed to the triggers. The **scope** must be a valid RON object with two keys: `documents` and `content`.

**`documents`** (string) must be a valid pattern object, and matches the URL of documents. For example, for a IEQL query to be only performed on `.net` or `.org` domains, `documents` would be set to match the pattern `(https?:\/\/)?(www\.)?[a-z0-9-]+\.(net|org)(\.[a-z]{2,3})?`. It seems complicated, but that's the power of RegEx! To match all URLs, set `documents` to the RegEx pattern object representing `.+`.

**`content`** may be one of `Raw` or `Text`. (Additional values are possible; refer to your implementations' source code for more information about the available content types.) `Raw` means that the triggers will be fed the raw document content. `Text` means that the trigger will be fed a cleaned version of the document with only its text (note that this functionality is only available for some content types; if the raw text cannot be extracted, the program will be provided the equivalent of `Raw`).

An example scope definition might be as follows:

```ron
Scope (
  pattern: {
    content: ".+",
    type: RegEx,
  },
  content: Text
)
```

This scope would consider all Internet documents to be in scope, and would feed the preprocessed text to the triggers.

#### Threshold

The **threshold** defines what triggers are required in order for the document to be a match. The threshold object must have the root keys `considers`, `required`, and optionally `inverse`.

**`considers`** (array) lists the various triggers and/or other threshold objects that should be considered. Triggers are identified by their IDs (strings), while other thresholds are themselves valid threshold objects. In this way, a threshold object may itself contain other threshold objects. (See below for an example.)

**`requires`** (integer) defines the minimum number of objects listed in `considers` that must evaluate to `true` in order for the threshold to be met (and the IEQL query to match). For an `OR`-like relationship between `considers`, `requires` should be `1`. For an `AND`-like relationship, `requires` should be the total number of objects in `considers`. If `requires` is greater than the number of objects in `considers`, the threshold will never be met; conversely, if `requires` is `0`, the threshold will _always_ be met.

**`inverse`** (boolean) determines whether the result will be inversed. With the inclusion of `inverse`, _any boolean expression can be expressed in IEQL_.

If three triggers are available, `A`, `B`, and `C`, we can define a threshold to match whenever any match as follows:

```ron
Threshold (
  considers: [Trigger("A"), Trigger("B"), Trigger("C")],
  requires: 1,
  inverse: false,
)
```

Alternatively, if we want _all_ triggers to be required for a match, we can write the following threshold:

```ron
Threshold (
  considers: [Trigger("A"), Trigger("B"), Trigger("C")],
  requires: 3,
  inverse: false,
)
```

If we want _no_ triggers to match—i.e., a match is defined as whenever _no triggers_ are activated—we could define the following threshold:

```ron
Threshold (
  considers: [Trigger("A"), Trigger("B"), Trigger("C")],
  requires: 1,
  inverse: true,
)
```

Finally, if we want to define a threshold in which `A` _must_ match _and_ either `B` or `C` (or both) must match, we can write the following threshold:

```ron
Threshold (
  considers: [Trigger("A"), NestedThreshold (
    Threshold (
      considers: [
        Trigger("B"),
        Trigger("C),
      ],
      requires: 1,
      inverse: false,
    )
  )],
  requires: 2,
  inverse: false,
)
```

Threshold composition is very powerful!

#### Response

The **response** defines the type of data that the IEQL query will return. It must have two keys: `type` and `include`.

**`type`** is either `Full` or `Partial`. `Full` indicates that each match of the IEQL query should be its own IEQL response, and no MapReduce-style operations should be performed on it. (In order words, it should be piped directly into the database.) `Partial` indicates that the IEQL query should return an IEQL partial response, which can then be aggregated.

**`include`** specifies the information that should be included in the IEQL response. It is an array of strings. The following includes are supported:

* `Excerpt` (full only)
* `Url` (full only)
* `Domain`
* `Mime`
* `FullContent`

#### Example Full Query

```ron
Query (
    response: (
        kind: Full,
        include: [
            Excerpt,
            Url,
        ],
    ),
    scope: (
        pattern: (
            content: ".+",
            kind: RegEx,
        ),
        content: Raw,
    ),
    threshold: (
        considers: [
            Trigger("A"),
            NestedThreshold((
                considers: [
                    Trigger("B"),
                    Trigger("C"),
                ],
                requires: 1,
                inverse: false,
            )),
        ],
        requires: 2,
        inverse: false,
    ),
    triggers: [
        (
            pattern: (
                content: "hello",
                kind: RegEx,
            ),
            id: "A",
        ),
        (
            pattern: (
                content: "everyone",
                kind: RegEx,
            ),
            id: "B",
        ),
        (
            pattern: (
                content: "around",
                kind: RegEx,
            ),
            id: "C",
        ),
    ],
    id: Some("Test Trigger #1"),
)
```

Minified, this query would look like this:

```ron
(response:(kind:Full,include:[Excerpt,Url,],),scope:(pattern:(content:".+",kind:RegEx,),content:Raw,),threshold:(considers:[Trigger("A"),NestedThreshold((considers:[Trigger("B"),Trigger("C"),],requires:1,inverse:false,)),],requires:2,inverse:false,),triggers:[(pattern:(content:"hello",kind:RegEx,),id:"A",),(pattern:(content:"everyone",kind:RegEx,),id:"B",),(pattern:(content:"around",kind:RegEx,),id:"C",),],id:Some("Test Trigger #1"),)
```

#### Filetype

IEQL queries typically end with the file extension `.ieql`. Using query files that _do not_ end in `.ieql` will likely generate a warning by the IEQL interpreter.

## Philosophy

As you implement or compose IEQL queries, keep the following in mind:

- All the triggers are fed the same data; triggers may not have different scopes or preprocessing steps. If triggers require different scopes, they should not be part of the same IEQL query.
- All queries should be able to identify the specific part(s) of the document that caused them to trigger.
- You should _never_ allow non-trusted entities to create IEQL queries. If you are running IEQL queries on your own server, you should be careful which queries you run—after all, a malicious query could severely slow down your scanning infrastructure.
