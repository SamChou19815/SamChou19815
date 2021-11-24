# One Year as Developer Lead

Almost a year ago, I started to perform developer lead responsibilities at
[Cornell DTI](https://www.cornelldti.org). My first major task was to grade part of developers at
the end of the spring 2019 semester. A few days ago, I just finished grading all developers for
their performance in the spring 2020 semester, wrapping up almost a year of work.

I came into the position thinking that developer lead has the easiest issues to deal with. I still
believe this statement is true for me. However, I didn't realize that the easiest issues are still
issues of a complex system full of
[Byzantine faults](https://en.wikipedia.org/wiki/Byzantine_fault). Easy DevOps issues become
exponentially harder to deal with when there is a people factor. Just like software engineering,
sometimes it's all about compromises.

Therefore, I decided to write this blog post as both a reflection for myself and as documentation
for future leads.

## The grand plan

On August 16, 2019, I finished my 12-week internship at Facebook. In the same month, our sibling
team published a
[paper](https://cacm.acm.org/magazines/2019/8/238344-scaling-static-analyses-at-facebook/fulltext)
on ACM that discusses how Facebook applies advanced static analysis at scale. The result is
mind-blowing and I started to think about whether I can apply even a subset of these techniques to
my side projects. Luckily, towards the end of the internship, I began to touch more CI related code,
and learned a lot of best practices on CI/CD on a monorepo scale.

The newly-gained experience has led to a complete refactoring of my website repo. It became an
automation-powered monorepo at the end of August. Then I started to turn my focus onto the
[`cornell-dti`](https://github.com/cornell-dti) GitHub organization, which I belong and had to
manage it soon. I want developers at DTI to _move fast on a stable infrastructure_.

## Phase 1: Fall 2019

An initial scan of all the repositories produced some quite disappointing results. Out of the 6
active projects at that time, only 2 projects enabled branch protection. Linter configurations have
been setup a year ago, but a quick experiment showed that almost no one respected these linter
warnings and errors. Continuous integration and deployment was only enabled on one project, which I
was its TPM in spring 2019.

If all the repositories in `cornell-dti` are my side projects, I would just self-impose a new
feature freeze and cleanup the code first. As a TA who witnessed projects with messy code ending up
getting very bad grades on correctness, I had a strong dislike of bad code. Yet, a large scale code
cleanup that blocks all feature development was simply impossible: it would cause a tremendous
amount of friction both on the development side and the people side.

Therefore, a gradual improvement plan is formed. The initiative was announced at the first TPM
meeting. During the first few weeks, all I did was to enable branch protection to prohibit pushing
to `master` directly. It doesn't even have to pass CI or even include CI tests yet. Luckily, changes
in this stage don't cause any pushbacks, and I went on to the next stage: re-introducing linters.

Making subteam properly configures their linters was initially a task I assigned to every TPM.
Sadly, nothing really changed in a week, since it's the time of recruitment and semester planning.
To prevent this task from never getting done, I decided to hack them together by myself. Let there
be CI checks. Then at week 3, we have CI checks for all active projects.

I anticipated that the planned change of making code review required for merging in pull request
will cause a big pushback, so I decided to provide a good infrastructure first. To be fair,
responsibly review each other's code with GitHub alone is not a pleasant experience. Of course, you
need to read the diff to catch some implementation strategy issue, but it's also important to check
that the implementation actually works. In the context of DTI, this means stopping your work,
checkout to another branch, and potentially nuking `node_modules` and reinstalling it. In OS
terminologies, it's a big context switch. Still, code review is important. If linters and type
checkers can catch all problems, then halting problem will become decidable.

The problems listed above already imply a technical solution: making code review less painful. More
specifically, making play-testing our apps less painful. Luckily, the problem is already
half-solved. Our apps are mostly deployed on Heroku or Firebase. Heroku already provided a feature
called review apps, which can provide a sandboxed execution environment for each pull request.
Taking the inspiration, I prototyped a
[similar solution](https://github.com/cornell-dti/samwise/pull/285) based on GitHub pages deployment
separated by folders, for Firebase web apps that do not support review app infrastructure. After
this new automation, required code review finally became a tractable problem. At week 4, my
envisioned strict development workflow was eventually adopted by all active projects:

> Required passing CI checks, required reviews for every pull request. Merge every pull request with
> confidence.

This was still not my complete plan, since it only solved the problem of code quality on a surface
level. However, this alone deserved some celebration, so I decided to document this change and use
it to fulfill my technical communication requirement at Cornell. You can see my written (LaTeX)
report [here](/pdfs/2020-05-14-one-year-as-dev-lead/technical-communication.pdf).

A superficial victory was achieved within the first month of the fall 2019 semester. However, it
took an entire semester to consolidate it. Although code review is required, there is no way to
_automatically_ tell whether an approving review is a result of deliberation, a simple showcase of
friendship, or even worse, a manifestation of a rushed release.

To automatically and completely solve the problem of careless review, we have to up the required
number of reviews to 3. Such a requirement is only enforced on release branch merges. The sad
reality is that most of the developers don't review each others code, and the job has completely
fallen onto TPMs, and sometimes even onto developer leads. It is possible to make code review part
of grading rubrics so that developers are forced to do them. However, I think it's unfair to
suddenly change grading rubrics when the semester is already halfway through.

So here comes the recurring theme in this blog post:

> Software engineering is all about compromises.

I first heard about this sentence from my mentor during my Facebook internship. Now I started to
live with it, and was about to make more compromises next semester.

## Phase 2: Spring 2020

I performed an analysis over all pull requests made in fall 2019 in the winter break, and found two
major issues:

1. A lot of pull requests are ignored for a long time;
2. A lot of pull requests are too big to effectively review.

I tried to solve problem 1 via the newly released GitHub's scheduled reminder feature, which can
post slack notification about unreviewed PRs in a specified time interval. After some pushbacks, the
reminder interval has been reduced from every day to every Tuesday and Friday.

After a discussion with the other two developer leads [Laura](https://github.com/lsizemore8) and
[Jagger](https://github.com/jboss925), we settled on a somewhat risky approach: strictly ban
extremely large pull requests, and setup bi-weekly developer portfolio to strongly encourage
developers to produce smaller pull requests.

To automatically enforce smaller diffs, I created a bot that can compute the number of significant
lines from a GitHub pull request. It goes beyond excluding something like `yarn.lock`. It can also
detect moved lines! If you are interested, you can check the project
[here](https://github.com/cornell-dti/big-diff-warning).

The rollout of the change was half good and half a disaster. I initially announced it in the first
biweekly DevSesh, and posted an `@channel` message in our `#dev` channel. Clearly, I had made the
mistake of assuming all information can get through, completely ignoring the number one question I
learned from distributed computing: what if some processes fail.

It turns out that the distributed system has crash failures and omission failures. Crash failure
refers to the cases when some developers just missed the first DevSesh, and we never relayed the
announcement again. Omission failure occured when our announcement was lost in a sea of other
announcements. In April, I did an `O(n)` scan of all submitted portfolios, and found that half of
the developers didn't satisfy the minimum requirement yet. As a result, we had to extend deadlines,
offer clarifications, and reach out to individual developers who seem to need help.

The final result was not completely satisfactory. Although most of our pull request changes are
under 500 lines of code, it is still far from industry's best practices to strive for diffs that are
less than 200 LOCs.

## Beyond DevOps and code quality

I was selected as a developer lead with a focus on DevOps, so my primary attention was on our
infrastructure and code quality. Nevertheless, I still did quite a lot of development within DTI,
excluding the ones related to CI:

- I helped [Flux](https://github.com/cornell-dti/campus-density-android) with their conversion from
  Java to Kotlin, and made the code more idiomatic Kotlin.
- I helped [Samwise](https://github.com/cornell-dti/samwise) with data layer refactoring that
  greatly improved pattern-matching developer experience.
- I helped [Queue Me In](https://github.com/cornell-dti/office-hours) with Firebase migration and
  they eventually shipped the version that contains a lot of my diffs.
- I helped [CoursePlan](https://github.com/cornell-dti/course-plan) to rewrite their requirement
  computation algorithm to avoid an `O(n)` client-side fetch of Cornell's course API.
- I helped [CU Reviews](https://github.com/cornell-dti/course-reviews-react-2.0) with their
  migration away from Meteor.

I also taught some optional DevSeshes on the topics I'm really passionate about:

- [How to build your own programming language](https://developersam.com/build-your-own-programming-language.pdf)
- [How to build a (simplified) React](https://developersam.com/build-simplified-react.pdf)

## Received feedbacks and final thoughts

Near the end of the 2019-2020 academic year, we sent out a developer survey and leads review form. I
am glad to see that our developers liked the strictified setup and we completed the first step to
transform our development from a do-whatever-I-like style to a workflow that follows established
industry standards.

I did get some feedback on the grading rubrics and my obsession with good code. The content above
served as my justification, but I still strongly believe that these are valid concerns.

When I started the initiative of code quality improvement 8 months ago, I set a little goal for
myself:

> By the time my term ends, the code quality should be good enough that I can comfortably work on
> every subteam (with the same level of obsession of good code).

Now I declare this goal to be 95% achieved. We are finally in a place to _move fast without breaking
things_. I'm proud that my work in the past year has unlocked several previously unimaginable
possibilities:

- Required TypeScript setup for new projects
- Required frequent peer code review for each developer
- Crowdsourcing DevSesh topics
- etc

Now it's the time for others to inherit the legacy and build new things on top of it. I will happily
step back and become a developer again, enjoying the infrastructure that I built.
