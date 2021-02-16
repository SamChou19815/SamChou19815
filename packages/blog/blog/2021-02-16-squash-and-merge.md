---
title: The Case for Squash and Merge
tags: [thoughts]
---

There are several things I have very passionate opinions on, like whether opening braces `{` should
be on a new line, etc. _(Answer: NO NO NO!)_ Merging in a pull request using squash and merge is
among the list. However, unlike the braces argument which is largely aesthetic, I believe that the
practice of squash and merge provides significant productivity boosts compared to merging by merge
commit.

![Squash and merge](/img/2021-02-16-squash-and-merge/squash.png)

<!--truncate-->

## Git as a Graph

Before we go deeper into the argument, we first need to understand some git internals to appreciate
the difference between the two merge options.

If you are a git beginner, you might perceive git as a sequence of commits, since you only push to
`master`. If you know more about git, then you might know how to use branches, and `master`/`main`
is just one of the many branches. However, it's important to note that branches are not the
fundamental structural unit of a git repository. Instead of thinking of the relationship between
branches, we should think of the git repository in a whole as a DAG (directed acyclic graph).

Let's consider the typical case when you have some feature branches, and then you merge them into
master. Here might be the picture before you merge it into `master`:

```text
o ------------ o ------------ o ------------ o (origin/master)
 \
  \
   \
   x ------- x ------- x ------- x ------- x (lsp-autocompletion)
```

If you merge by merge commit, then both the last commit of `master` and the last commit of the
feature branch will be the parent of the merge commit:

```text
o ------------ o ------------ o ------------ o ----- o (origin/master)
 \                                                  /
  \                                          -------
   \                                        /
   x ------- x ------- x ------- x ------- x (lsp-autocompletion)
```

If you merge by 'squash and merge', then all the commits in the feature branch will be squashed into
one commit and added as the latest commit in `master`:

```text
o -- o -- o -- o -- o -- x (origin/master)
                         |
                         |
                    (from lsp-autocompletion)
```

There is also a third option on GitHub, which is 'rebase and merge'. It will change the base of the
feature branch to be the last commit on master, and then add these commits to master:

```text
o -- o -- o -- o -- o -- x -- x -- x -- x -- x (origin/master)
```

## The Case for a Linear History

Let's first examine why the default option on GitHub, merging by merge commit, is not a good option.

As I said in the previous section, git is a graph, and more specifically, a directed acyclic graph.
The git graph can provide some partial order on commits. For example, if both commit A and commit B
are both in the same branch, and commit B is a descendant of commit A, then we can write `A < B`. If
a branch is branching off `master` at commit A, then for all new commits `C` in that branch, we have
`A < C`. Finally, we can take the transitive closure of `<` to get more ordering of commits.

```text
o1 ------------ o2 ------------ o3 ------------ o4 ----- o5 (origin/master)
 \                                                      /
  \                                              -------
   \                                            /
   x1 ------- x2 ------- x3 ------- x4 ------- x5 (lsp-autocompletion)
```

Putting aside the mathematical language in the above section, for the graph above, we know this
partial ordering:

```text
o1 < o2 < o3 < o4 < o5
o1 < x1 < x2 < x3 < x4 < x5 < o5
```

It's important to know that the ordering is **partial**. For example, we don't really have a way to
compare commit `o2` and `x3`.

You might wonder why I bother talking about the graph and ordering, since all you need to know about
git is how to commit, how to merge, and how to solve merge conflicts. It turns out that, your job is
much more than adding nodes to the git graph. You are also responsible to understand the graph. In
plain English, _you are also responsible for understanding other people's work_. Therefore, reading
is almost as important as writing.

To read other people's work, you usually read on GitHub in a commits page like the following instead
of jumping around the graph in Git CLI.

![Commits Page](/img/2021-02-16-squash-and-merge/commits-page.png)

The commit page shows you the commits as a sequence. However, I just told you that you simply can't
totally order the commits in different branches, so you might wonder how GitHub can do that. Well,
actually you can. You can order all the commits by timestamp, although this is not an ordering
implied by the graph structure.

Let's assume we are in an ideal world where people don't fake commit timestamps (they certainly
can). Then this is the order we get for the graph:

`o1 < x1 < x2 < o2 < x3 < o3 < x4 < x5 < o4 < o5`

Everything is nicely ordered, except that it doesn't make any sense. For example, you might think
that based on the ordering `x4` depends on `o3`, which is simply not the case. As a result, you are
essentially reading some shuffled together garbage.

Thus, for the sake of understanding git history, merging by merge commit is not a good idea. We need
a linear history that actually makes sense.

## The Case for a Squashed Linear History

In the last section, we establish that a linear history is superior to some scrambled together
branch history. This leaves us with only two choices: 'squash and merge' and 'rebase and merge'. You
might think that the point of git is that we can keep track of history. Since the 'squash and merge'
option destroys the commit history, it must be the case that 'rebase and merge' is better.

On the contrary, I would argue that sometimes destroying some part of history is a good thing.

It's important to note that all git commit histories are created equal, **but some are more equal
than others**. You might think that commits in branches usually denote some important milestones.
For example, in the first commit you implement parts of the feature, and then in the second commit
you implement the other half of the feature. Nevertheless, from my experience, such nice commits are
rarely the case. In reality, you have all kinds of junk commits in between, like this:

![Junk Commits](/img/2021-02-16-squash-and-merge/junk-commits.png)

The junk commits are not really important. Usually you do something wrong in the commits. Other
people who are reading your code probably don't care what you did wrong. They are more likely trying
to find in your commit what is the correct way of solving a problem, in which case only the final
squashed commit matters.

Such junk commits can also make the experience of `git blame` unnecessarily worse. For example,
instead of finding the commit that might explain the rationale of including a line, you see this
`fix linter error` commit:

![Fix linter error junk](/img/2021-02-16-squash-and-merge/fix-linter-errors-junk.png)

Of course, you can still click on the commit and it can lead you to the pull request. However, that
is extra work that can be easily avoided if you use 'squash and merge' in the first place.

Finally, the argument that the commit history is lost is simply not true, at least on GitHub. You
can always go to the pull request, and GitHub pull requests keep all the commit history even if the
branch is deleted. For example, these junk commits will be kept forever:

![Junk Commits](/img/2021-02-16-squash-and-merge/junk-commits.png)

Yes, it is extra work to go to the pull requests to see the full commit history, but the reality is
that the need for seeing the final result as a whole is much more common than seeing individual
commits as a process.

## Conclusion

Now we have arrived at the inevitable conclusion that squash and merge is the best option for most
use cases of software engineering, because it produces linear history that is useful and readable.
It's worthy to note that it's not my random personal opinion. A lot of high-impact open source
repositories like [Docusaurus](https://github.com/facebook/docusaurus/commits/master) and
[Babel](https://github.com/babel/babel/commits/main) both choose the same approach for the same
reason.

In the end, if you are convinced by this blog post, you can easily enforce squash and merge only by
unselecting all the other merge options in the repository settings.

![Merge Options](/img/2021-02-16-squash-and-merge/options.png)
