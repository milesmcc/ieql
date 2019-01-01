<p align="center">
  <h3 align="center"><img src="assets/ieql_logo.png" width="250px"></img></h3>

  <h4 align="center">
     An open standard for monitoring Internet content
  </h4>
</p>

---

This repository contains the specification and reference implementation for IEQL (Internet Extensible Query Language, pronounced _equal_). IEQL is an open standard for monitoring and querying Internet content designed to be fast, efficient, and scalable.

### What are queries made of?

IEQL queries have two parts: the _triggers_ and the _threshold_.

**Triggers** are individual queries, typically RegEx patterns. Triggers can be configured to match only on certain elements of Internet documents (such as `body text` or `HTTP headers`), and also to match on Internet documents at a certain location (for example, every document from domain `nytimes.com`).

The **threshold** are the compositions of triggers that are required in order for the query to match. For example, an IEQL might have three different triggers: Trigger A, Trigger B, and Trigger C. An IEQL query could be defined such that a match is defined as any time Trigger A fires and _either_ Trigger B _or_ Trigger C fires. Alternatively, the IEQL query's match threshold could be _any two triggers_. These trigger compositions are an important part of what makes IEQL powerful.

### Why are IEQL queries awesome?

IEQL queries provide three main features:

- **Speed** — IEQL queries can be _compiled_ so that they can scan millions of documents per second. They are _blazing fast_.
- **Grouping** — You can combine thousands of IEQL queries together and scan using all of them at the same time without serious performance trade-offs.
- **Openness** — IEQL is an open standard, which means that you can implement it however you'd like.
- **Extensibility** — Because IEQL is built on top of RON, it's extensible—and yet also backwards-compatible.

### How can I use IEQL myself?

To get started with IEQL, either use the reference Rust implementation or create your own based off of the [open specification](SPECIFICATION.md).

### Licensing

This document is licensed CC-BY-SA, &copy; R. Miles McCain 2018. The Rust reference implementation is licensed according to the `LICENSE` file.
