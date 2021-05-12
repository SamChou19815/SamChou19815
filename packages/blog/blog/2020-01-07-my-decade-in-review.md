---
title: My Decade in Review
---

Following the lead of React and JavaScript god [Dan Abramov](https://overreacted.io/), I decided to
also publish my decade in review blog post.

I want to clarify that I intend to present the following content as plain facts. I will talk about
the facts that certain events happened, and the facts that I thought and reacted to certain events
in certain ways. You should treat them as a totally ordered set of boring logs rather than
definitive guidance to something.

(To be friendlier to not-so-technical readers, I _italicized_ all the technical jokes that are not
essential to understand the story.)

<!--truncate-->

## 2010: Nothing Big

People have different opinions on whether 2010 is in the decade that contains 2019 or the previous
decade. I think there is no significant event for me in 2010, so it doesn't matter for the
discussion. Therefore, let's avoid the debate and go straight into 2011.

## 2011: Killing the Class BBS

I was enrolled in Huayu Middle School during 2010FA to 2014SP. To my non-Shanghainese friends, it
was the best middle school in Shanghai, judging from raw scores and high school admission. To my US
friends, it matches your stereotypical impression of Chinese education. During the four years, there
were ups and downs for me, but my exam scores stay in the middle on average. Therefore, I claim that
I gained something other than pure academics that might be experienced by the top students or pure
pain experienced by the kids who did poorly on exams.

![Unrelated: Gossip in Math Paper](/img/2020-01-07-my-decade-in-review/gossip-math-paper.png)

In May 2011, gossip on the class teacher was spreading like a virus. _Just like a configuration
error was amplified by gossip protocol that eventually took down the entire AWS_, the discussion of
that particular gossip went from offline to online. To understand the power of gossip, the above
screenshot of a math paper might give you some rough idea, and you can read the full 1972 paper
[here](https://www.math.uni-bielefeld.de/~sillke/PUZZLES/gossips.pdf). We did have a class BBS at
that time, and it was not hard to imagine that more discussion appears over there and eventually the
class teacher took some action. On May 20, 2011, she mentioned it in the morning class meeting and
banned further discussion.

Initially, I decided to lead a boycott of the class BBS to protest this ban. Soon, I found a better
approach: creating an alternative. On the same day, I relaunched my own BBS and publicly marketed it
on the class BBS as the alternative that permits free speech. After a few hours, my own BBS's
popularity surpassed the official one, and by the end of the semester, my action turned the official
class BBS into a ghost town. The success was short-lived. People's passion for free speech and the
complete absence of regulation soon faded. At the end of the year, my BBS was effectively dead.

Newer platforms of communication emerged. BBS, the technology of the last central, cannot survive
the wave. However, I am still proud that it appeared to be the case that I killed the class BBS, not
the force of the era.

## 2012: An Almost Failed Attempt to Learn Programming

If you knew me, you might not be surprised to read that I created a BBS in the last section. You
might be surprised to see a class teacher who magically created a BBS. If you understand computer
science, you would know that creating a dynamic website like BBS is non-trivial: you will need a
database, server, domain name and more importantly, some knowledge of programming. At that time, I
had none. But there was a platform called UUEasy in China that allowed non-technical people to
easily create their BBS for free. The actual UUEasy site was dead a lot time ago, the following
screenshot came from a design company that once designed its homepage:

![Already dead UUEasy](/img/2020-01-07-my-decade-in-review/already-dead-uueasy.png)

The major difference between my class teacher and me is that I wanted to dig deeper. The official
class BBS almost used all default settings, while I spent a lot of time finding and applying all
customizations. Soon, I found the set of limited customizations cannot satisfy my desire of creating
things, so I need something more powerful.

In mid-July, 2011, I correctly concluded that power comes only from code and I bought my first
programming book in C++, but almost gave up. In the next 10 months, I most typed some Visual Basic
code from books into the IDE, copy-pasted some random snippets, run it and called it a day. I did
not understand these algorithms, nor why the code can work.

Starting in April 2012, things started to change. I started to seriously learn web development and
managed to create a simple login system with PHP backend and MySQL database. At the moment when my
first test AJAX request sent from frontend reached a local backend, read some data from the database
and sent something back to frontend, I knew that I could truly create things on my own.

In this period, I created a lot of useless web applications. In the process, I learned PHP,
JavaScript, and Java. These random activities probably gave me 5000 to 10000 lines of code
experience. The code was terrible and thankfully forever lost, but it was a necessary first step.

## 2013: DEFCON 1 amid a Period of Hostility

DEFCON 1 could be the status I used to describe that period, but it was a system developed by me at
the time, in response to events that I would classify as DEFCON 1.

![Steam Game DEFCON](/img/2020-01-07-my-decade-in-review/defcon.png) Source: Steam Game DEFCON

The system was a _2FA-like_ login system. The first login was a normal one, which was implemented
with a simple Google login at that time. The second factor showed its uniqueness. The second step
login requires a password that was regenerated every time a login is requested. Only the
administrator of the system can read the generated password and send it to the person who originated
the login. The system was designed with data security in mind even under an interrogation that
involves torture. The user must communicate to the system administrator via another channel, and use
a pre-settled password to obtain the second-step token. Unlike the conventional time-based 2FA
technique, there were two passwords: one for normal operations, and the other for self-destruction.
Once the user under the prosecution decided not to take the torture anymore, he invoked the protocol
with the self-destruction password that is unknown and untestable by the interrogator, and all data
would be automatically wiped. Of course, to satisfy the safety property, the system administrator
must be trustworthy and could never be interrogated by potential attackers.

![Example Flow of DEFCON 1](/img/2020-01-07-my-decade-in-review/defcon-1-flow.png)

The origin of my intention to create such a system could be traced back to 2012. Our class teacher
was replaced in October 2012, but the impact only began to reveal itself during this period. My
classmates had been increasingly discontent with the new class teacher's style of management. In
December 2012, there was an incident when a cell phone rang during a math class but no one admitted.
Later, I happened to witness that the class teacher ordered some students to search for other
students' bags to investigate. I was deeply disturbed by this intrusive way of investigation, to an
extent that the story eventually went into my college admission essay as a flashback.

A deep sense of mistrust is growing and I started to think about what I could do with my knowledge
of programming. Starting in May, every new system I implemented had a one-button-click
self-destruction feature that wipes the entire database built-in, in case that those cell phone
search like incidents happen again. I open-sourced all these self-destruction code as a form a
protest, to show that I am more willing to losing all my data rather than giving away any single
bit.

Thankfully, the self-destruction protocol was only invoked during local system testing, and never
invoked in production, although it was running with DEFCON 1 alert until the end of my middle
school.

In hindsight, I might have built something way more powerful and complex than necessary, but I am
glad that I learned some system design and basic cryptography along the way. This was also my first
attempt to fight the big brother with technology.

## 2014: A Year of No Significance

This year of no significance could be divided into two boring halves: the middle school half and the
high school half. Two halves both led to a dead-end somehow.

The middle school half was all about exams. It was the second semester of ninth grade, where we were
supposed to give up all entertainment and completely focus on the preparation of the high school
entrance exam. Everyone was crazy optimizing themselves for the exam. Just like any other similar
story, the exam eventually came. Someone was happy, and some others were not. Eventually, things
were settled and everything was over. _The system had reached eventually consistency since there
were no additional updates._

In my private note, I commented that the final graduation ceremony marked a point where the
civilization had gone with the wind. This was not a glorious civilization, but a deeply flawed and
skewed one as a direct result of the Chinese educational system. Yet humanity still shines: unlike
some stereotypical impression against the best exam-oriented school, friendship, and during some
period, a common enemy did exist. It's just not a result of the system, but because of the wonderful
people. Believes and dreams still refuse to die even under the bad system.

Luckily, non-academic activities continued to exist. I did read Nineteen Eighty-four in this period
and I was glad that this year was not as bleak as 1984 and the clock never striked thirteen.
Rebellions still existed, but now in a different form: some students decide just not to take those
stupid exams but to study abroad. I was among that group to study abroad but still took the final
high school entrance exam since I need the score to get into a better class in an international
school. To me, the system was a dead end: a society that could only do one thing well should not be
considered as a society. _Instead, it should be abstracted into a function within a Docker container
and let Kubernetes orchestrate it._

The high school half was not that depressing. It was mostly a period of exploration when all the
explored paths had reached a dead-end. One such explored path is the student union.

I was admitted to the publicity department to help them make videos. Ironically, I made **zero**
during the entire time. In the middle of the semester, I was asked to help improve their website to
sell charity tickets, only to find out the work is completely unnecessary to them. In summary, that
is one semester with no real work done.

Before joining the union, I was promised, either explicitly or implicitly, that it was an
organization of the students, by the students, for the students. It would also have enough resources
to help me achieve things. The only small problem is that none of the above is true, at least from
my perspective.

![None is True](/img/2020-01-07-my-decade-in-review/none-is-true.png) None is True. Source: CS 5414
Lecture [Slides](http://www.cs.cornell.edu/courses/cs5414/2019fa/notes/Week5.pdf)

Events were still organized; student lives were said to be enriched; big impacts were made. Yet,
their way of organization and work showed no sign of being revolutionized by the latest technology.
Yes, promotion videos are made by computer instead of movie tapes, but the video team still
collaborated by sending each other things like `final_final_version_4-12.mp4`. I was shocked to see
their leader being proud of having all sorts of messy files randomly laid on their computer desktop.
Apparently, a peiceived fake passion of work was more important to use technology to solve problems
efficiently. Therefore, I was not surprised to see my suggestion of automating their workflows being
appreciated and silently ignored.

I did not want a career like this, so I sent my resignation email in February 2015.

2014 was a year of no significance. On the surface level, there was an exam, a graduation ceremony,
a new school, and a potentially different experience from the student union. The society runs as
usual. However, all the paths towards perceived success had been closed **by** me. I longer wanted
to continue bearing with the exam-oriented system, and I never wanted to get involved with
bureaucracies like the glorious student union to "do community service and to practice my leadership
skills". However, it was not as depressing as the year 1587 for the Ming dynasty. My solution is
simple: going back to the root and follow my passion. As a result, a new chapter would unfold in
2015, the midway year.

## 2015: Computerization

In February 2015, I was invited to join the club called Computerization that was just founded last
semester. Computerization developed a platform for teachers to send and for students to receive
homework. I was the main developer from February 2015 to June 2016 and was the club president from
September 2015 to June 2016. I already covered most of the story in
[Computer Science in High Schools](/2018/12/31/cs-in-high-schools/) and covered the same story from
a more technical perspective in
[The Road to Better Engineering](/2019/08/17/the-road-to-better-engineering/). For the sake of
avoiding duplicating content and code, I decide to leave readers to read those posts first as an
exercise. However, I do want to highlight an event that was not discussed in detail before. To avoid
discrepancies, I decide to directly quote my words in my college admission essay:

> "Please implement a feature to check whether students have actually seen their homework," a
> teacher asked.
>
> This was the fifth time I had received a request to add a feature to our platform, which allows
> teachers to post assignments for students online. We had launched the beta version of our platform
> for both teachers and students, hoping to get their advice to quickly improve the platform. We
> initially invited about 10 teachers to our platform; however, due to our slow progress in changing
> the code, only three teachers remained. We wanted to make quick modifications to retain our
> teacher users, but as a computer science guy, I was instantly aware that making this change would
> require tracking users' scroll bars, which can be used to re-visualize exactly what's happening on
> a user's screen, almost equivalent to quietly installing malware to record our users' screens when
> they are using our platform.
>
> "You know, the students are making up excuses for not doing homework," the teacher said.
>
> My memory flashed back to an incident in middle school. In the middle of a math class, there was
> an unexpected ring of a mobile phone. The math teacher angrily demanded the student, we never knew
> who it was, admit his or her error. There was no response. Later, the homeroom teacher ordered
> some of my best friends to clandestinely search for the student's schoolbag in the suspected areas
> of the ringing phone. All of my friends thought that was bad, but they still obeyed the order
> because they were afraid of the teachers' criticism. I don't know if I had been chosen whether I
> would have challenged the teacher, but I think I would at least have chosen not to tell the
> teacher if I did find evidence of which phone was ringing. If I was not brave enough to blatantly
> reject the request, at least I could have resisted passively.
>
> "So that I can check whether they are lying," the teacher explained.
>
> ...

To grasp the situation entirely, I might need to have more background. The main homework feed of the
student side is rendered as a waterfall card UI like the following screenshot.

![SAM Screenshot 2015](https://developersam.com/static/media/sam-first.566cbe7e.png)

To accurately decide whether a student has seen a homework, at least the bottom-most reached
y-scrollbar needs to be tracked and sent back to the server. I feel very uncomfortable about
tracking scrollbars and especially sending the data back to the backend. Our code is open-sourced.
Everyone on earth can figure out we did this damn thing. Even if we choose to close source our app,
people can still figure out by reading the JavaScript that must be sent to the browser. I eventually
wrote this sentence in my college application essay:

> I didn't want to be the one who instilled fear into every student, including myself, that an
> "inappropriate" movement of the scroll bar could be used as evidence against him or her.

There exist alternatives that look less bad for PR. For example, we might require students to click
to load the full details of homework, and we can record the GET request for loading additional
details in the database with the pretext of analytics. Then, we could supply the data to the
teachers when they requested. I still hated this idea. It would make the app significantly harder to
use, and it would be an instance of restructuring the app for the sake of easy surveillance. As the
one who built interrogation-threat tolerant systems before, I would not accept this kind of feature
into the system built by myself.

The event occurred around May 2015, before I became the club president. Since I was the only
developer in the team, I had the de-facto veto power and invoked it.

The event went through without any real repercussions, but its significance to me keeps getting
bigger. It was the first time I said no to a teacher's request. It was probably the first time in my
school when technology won over politics because the political side was too incompetent to implement
the same thing on their own. It was my last chance for me to reverse my personality to lead the
project to greater success. I'm **glad** that I missed it.

When I stepped down in 12th grade, the platform looks like the following screenshot. As you can see
technical improvements and change of user interface did not move things to much.

![SAM Screenshot 2016](/img/2020-01-07-my-decade-in-review/sam-later.png)

Another big ongoing struggle in that year was my fight with the student union. Although never
succeeded, I designed our platform to be a drop-in replacement for student unions' publicity
channel, only to find out later that at least in my year, other students used neither. I planned and
executed our recruitment ahead of the student union's recruitment to steal the pool of talent, only
to find out that we could only get one good CS student during recruitment.

More importantly, I tried to make Computerization anything that is not the student union. I was the
master hacker instead of the work distributor. I wanted to step down from the position of president
to focus more on technical aspects, although the attempt failed since no one else wanted to be club
president. I also strived for independence for the club very hard, using my own money from
scholarships to fund it instead of asking for the school's assistance. I am not claiming these are
good leadership characteristics. Some of them are not. For example, being the master hacker resulted
in a period of chaos when I entered the 12th grade and no longer had time to do the hacker work.

![Master hacker to fail](/img/2020-01-07-my-decade-in-review/bad-master-hacker.png)

These efforts to challenge the big brother were eventually proved useless. _The distributed system
reached eventual consistency_, where the student union was still the big brother and Computerization
was still one of the many non-special clubs. However, I was glad that I made some successful noises.
One later year, Computerization grew from a club of nowhere to an organization of over 30 new
recruited members, although many of them had little or zero programming experience that led to
another disaster.

## 2016: My Path to Cornell, as a Nerd

In the first quarter of 2016, it became clear that the product of Computerization had reached
another dead-end. I had no idea how to fix it since I was not a PM and never intended to be one. I
thought that it was a complete failure: in the process, I never learned any new technology, and I
did not get the desired conventional success as a return.

It turned out that it was indeed a disaster on the product side, not so much for my CS career. It
told me what I liked and disliked. In May 2016, I read the famous
_[Why Nerds are Unpopular](http://www.paulgraham.com/nerds.html)_ essay by Paul Graham. This essay
systematically examined the culture of an average American high school and its impact on nerds.
Nerds are discriminated against, prosecuted, yet they remain who they are.

I have already discussed the impact on the article on me in the post
[Computer Science in High Schools](/2018/12/31/cs-in-high-schools/), so I will omit further analysis
here. What has not been mentioned before is that I happened to find this article and read it right
before our school's trip to Inner Mongolia for a week of relief and cultural understanding. To be
fair, I still did not understand what was the actual point of the trip, so the above statement is
just my best-effort guess.

I correctly conjectured that I won't enjoy the trip, so I pre-downloaded a lot of machine learning
lecture videos and brought a linear algebra book with me. During a week there, I was reading classic
ML papers and watching Andrew Ng's videos to figure out _why the heck my classifier's training
accuracy stuck at 40%_, while other students were playing board games at night. Against the wishes
of anyone who wanted to convert me into a more "well-rounded" person, I was very proud of this
openly nerdy act of defiance and even mentioned it in my graduation ceremony speech.

The week involved a lot of driving, and it was very uncomfortable to read the linear algebra book or
_think about the impacts of different activation functions to neural networks_ on the bus, so I had
a lot of time to silently reflect. I remembered the discussion about the desire for popularity in
the Nerds essay:

> "But in fact I didn't, not enough. There was something else I wanted more: to be smart. Not simply
> to do well in school, though that counted for something, but to design beautiful rockets, or to
> write well, or to understand how to program computers. In general, to make great things."

On Wednesday night, I recalled that there was a ceremony for being grown up. Now I almost forgot
everything about those random poems and songs, but only a student film that showed the life of one
nerdy student, that eventually became more socially natural. I was not sure whether it was a satire
against me, but I no longer cared. I enjoyed watching it because it's fun to see it getting so
wrong. I thought that there would be a day that my six-figure salary would render all these talks
futile.

Of course, I didn't get a six-figure salary that year, nor there was any evidence that I was on
track at that time. However, I did come back from Inner Mongolia with a resolution to remain nerdy
and a workable idea to fix my neural network models. I did learn something profound about myself,
which could make the event organizer happy since it probably was one of the goal of the trip,
although it was in an opposite direction that people might hope.

In August 2016, I had a tour around various US Universities' campuses. When the tour was planned
around May, its original purpose was to learn the culture and background of different universities
to help me to decide which one to apply for early decision. Things had changed. In June, I already
locked myself to Cornell after some initial research on the CS courses offered by some universities.
In July, my Mom insisted that Penn was better and sent me a list of graduation reports of Penn's CS
department with quite a lot of graduates entering Google. Cornell did a bad job on publishing these
stats and it would take me a while to find a slightly outdated and incomplete version of Cornell one
to show that it's not worse. Things were in a stalemate and the tour was the key to break the
balance.

Fortunately, Cornell organized the tour by colleges and I was attracted by the college of
Engineering. Penn only had a generic one and it had its insuppressible business atmosphere that I
already strongly disliked after my interactions with the bureaucratic student union that shared the
same traits. My decision was made. _In the Paxos protocol, a process named `sam` had received `p2b`
messages from a majority of its internal acceptors and decided to put `'Cornell'` into slot `ZERO`
and it's now up to the learners to figure out the acceptors' decision._

Regardless of anyone's personal feelings, it was an extremely risky decision. No one even in my high
school had successfully entered an Ivy League's engineering school. _If college admission were a
machine learning optimization game, my decision would be overridden with only one epoch of gradient
descent, given a big pile of historical data against me._ Yet I recklessly went ahead, gambled and
triumphed. However, it was a little sad to learn that the same story could not be replicated by
later students from my high school.

## 2017: An Actual Year of No Significance

In 2017, nothing significant happened. I entered Cornell, but the decision came out last year. I got
rejected by Google without an interview, but it's a result of all previous years' ignorance on how
to be good at software engineering.

I reckoned whether my high school experience would be the peak of my career. Fortunately, events in
the next year invalidated the conjecture.

## 2018-Present: A Clean Reboot

I did not have a happy experience in the summer of 2018. Before the summer, I was rejected by AppDev
and even the web development subteam of ASCU, the CS student organization at Cornell that did not
develop any dynamic websites in the past. I was in doubt whether I could get any good internships
for next summer. In the mood of anxiety and uncertain, I decided to try something different:
compilers.

I wanted to create a programming language for a simulation game for my CS 3110 final project but
eventually decided that I did not have enough time to fight the OCaml parser generator. However, I
had a ton of time in the summer. On the plane back to Shanghai, I analyzed the difficulties of
implementing different stages of compiler and concluded that if I were not too ambitious about
language features, I could easily make one with my existing knowledge. Therefore, I got started. A
month later, a functional programming language that can be compiled to Kotlin was born. In the same
summer, I also contributed to open source and developed some frameworks for the upcoming hackathon
in fall.

![The final FB interview](/img/2020-01-07-my-decade-in-review/timeline-fb.png) Screenshoted from my
[website](https://developersam.com)'s timeline section

Probably due to these projects, especially the compiler one, I secured interviews from Google and
Facebook during the fall career fair and eventually got accepted by Facebook. In September, I was
constantly practicing LeetCode, and became so tired of algorithms at the end. In the same fall, I
was accepted into Cornell DTI as a developer, ending my repeated failures of getting rejected by
project teams. Finally, I was in an organization that did something close to the real industry.
There were code reviews, a real PM that was not forced into the position, and designers that created
designs that must be faithfully implemented. I enjoyed my time there. In Fall 2018, introduced
linter and type checker to my subteam to help improve the code quality, and was recognized as the
master hacker in the team. In the spring, I became the technical project manager for the team and
then spent more time improving the infrastructure of the project. In the fall, I became one of the
two developer leads and pushed the same infrastructure improvements to the entire team. Learning
from my experience, I did not seek any non-technical role within the team: I know I won't like it,
and I am more willing to accept the alternative of being a developer all the time. Thinking about
the future, I plan to step down to be a developer again once my developer lead term ends. I am ready
to be the master hacker yet again.

I also learned some of the early histories of my project team. It was originally a technology
subsection of the student assembly and we were originally called SATech. Then we broke up and
elevated ourselves to the status of a project team and let the student assembly rot. It did rot.
Last year, the presidential candidate of Cornell student assembly whose only campaign promise was to
add something impressive to his resume won the election, and immediately resigned since the campaign
promise was fulfilled. You can read more about this hilarious incident in
[the Cornell Daily Sun's article](https://cornellsun.com/2019/03/28/davis-dominguez-to-resign-from-student-assembly-after-losing-presidential-race/).
Student union or assembly, whatever it might be called, looks dysfunctional globally and
universally.

At the same time, I found my passion in programming languages and my compiler project continued
after summer 2018. _The original one in summer 2018 was abandoned for a complete rewrite with better
architecture in mind: good error messages and real type inference were taken into consideration from
day one rather than some afterthought. Recently, I was working on adding IDE support to my new
language samlang and hope it can bootstrap in the future years._ This is the reason why this blog
post was delayed for so long.

![Autocompletion for samlang](/img/2020-01-07-my-decade-in-review/samlang-autocompletion.png)
Autocompletion for samlang

I will not add any reflection for the past two years. It is too close to the present to derive any
useful insights.

## The End Of the Last Decade

I wrote down some of my previously unpublished stories, thoughts, jokes, and criticisms as a raw
log. I would expect disagreements because they make this article meaningful rather than a repetition
of some axioms or fake axioms that everyone is supposed to agree with.

Contrary to your expectations, there is no concluding section. I would not classify these
experiences as good or bad, but unique. I believe it's so unique so that you are bound to mess up if
you to imitate my path without thinking anything. It's like pushing code from StackOverflow directly
into production without even reading it, so don't do it.

Despite putting some of my potentially biased thoughts into this post, I still welcome your open
interpretation. Again, treat this as a piece of random code written by some unknown guy, understand
it, and copy what you need.
