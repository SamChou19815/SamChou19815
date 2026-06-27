# Developer Sam

```rust
/**
 * Copyright (C) 2015-2026 Developer Sam.
 * @demo https://samlang.io/demo
 * @github https://github.com/SamChou19815
 * @bsky https://bsky.app/profile/developersam.com
 * @resume https://developersam.com/resume.pdf
 */

import {List} from std.list;

class Developer(
  val github: Str,
  val projects: List<Str>,
) {
  function sam(): Developer = {
    let github = "SamChou19815";
    let projects = List
      .of("samlang")
      .cons("website")
      .cons("...");
    Developer.init(github, projects)
  }
}

class Main {
  function main(): Developer = Developer.sam()
}

```

Best viewed on [developersam.com](https://developersam.com).
