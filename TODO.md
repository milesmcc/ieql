# To Do

* Make multithreading more memory efficient (currently, queries are copied in memory for each thread)
* Make scope patterns smarter (right now, scope checking functionality is done at a per-query level, however this could be made more efficient if we used the built-in RegEx set checking)
* Make the `CompiledQueryGroup` pattern matching optimized for both `Raw` and `Text` queries. (Currently, only one is supported.)