---
title: 'Project DEFCON 1: A Confidential Data Storage System'
---

### 1.Introduction

This short article is devoted to describe a data storage system that ensures data confidentiality
even if you are interrogated under torture. I personally call the system Project DEFCON 1.

The system, with an option to switch on and off, lies behind the usual login feature as a
second-step authenticator. Since it dramatically increased the complexity of login process, it is
recommended to switch it on only when you feel imminent threat, as suggested by the name
'[DEFCON](https://en.wikipedia.org/wiki/DEFCON) 1'.

<!--truncate-->

### 2. Architecture

The system will automatically generate a new random string as the second-step password, which only
the person with admin access of the server can obtain. The person who wants to login must contact
the administrator via a secure tunnel to get the key and finish the login process.

The communication between a legit system user and administrator must follow a standard procedure.
The user can only send one of two strings: one for normal access, and the other for emergency. The
one for emergency must be misguided as a normal one. For example, if user Alice wants to access her
file containing all of her password, she must contact administrator Bob and say something like
"ABCDEFG" or a strong password when she feels safe. (See
[Alice and Bob](https://en.wikipedia.org/wiki/Alice_and_Bob)) In another day, Alice is held captive
by an evil guy named Eve who wants the evidence of her secret plan to defame the student government.
Under torture, Alice gives Eve the password for first step login. Then she contacts Bob and says "I
want the key". This message looks quite normal to Eve, but for Bob, it is a coded message for "I'm
in danger". Bob then wipes all the data.

As shown by the example above, the system takes the confidentiality of the data to extreme. The
essence of the architecture is that the action of sending coded emergency message has irreversible
effect. It may persuade the bad guy to stop torturing because it will not yield any progress.
However, personal security is never ensured in the process.

In order for the system to be absolutely secure, Bob must be the only administrator and must not be
under threat at any time. Bob may be your friend in another country, or if you are a student who
hates school surveillance, Bob can be a graduated alumni.

### 3. My Comments

The idea was developed and furnished almost four years ago but never openly published. I thought of
the system after an indirect but severe data leak. The incident made me seriously question my data
security infrastructure and pushed me to develop an overly aggressive and radical approach. The
system described here was a defensive approach, while my research in AI was designed to be an
offensive approach but proved futile.

As the project name suggested, the system was never activated just like DEFCON 1 was never reached
even during Cuban Missile Crisis. (It was activated several times on local testing server, of
course.) Probably it showed my determination on data security or I was just overly sensitive after
the incident. I hope it is still the case in the future.
