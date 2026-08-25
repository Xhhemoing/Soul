MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# LOCAL next-app prediction for Soul v0.2: literature and testability

## Bottom line

Soul should first test a ladder of **MFU/MRU → first-order Markov → variable-order n-gram/APPM**, with an empirical delay baseline if predicting *when* as well as *which executable*. These methods are confirmed-testable on one Windows machine from Soul's existing local foreground-session record and produce explanations that are counts a user can inspect and correct.

A regularized multivariate Hawkes model and a small temporal-only RMTPP are also testable as research comparators, provided the session-start event timestamp is retained. They are not justified as defaults until they beat the count baselines in chronological, per-machine evaluation.

The complete published systems are different:

* **Federated SeqMF + QHarmony is not a Soul v0.2 one-machine candidate.** Its claimed benefit depends on multiple users, a shared app-embedding server, gradient exchange, and a known installed-app candidate set. This conflicts with the local-only/default-no-egress lock before v0.4. A single-PC multi-client simulation can test code mechanics, but cannot test its collaborative or privacy-utility claim.
* **Full ATPP is not testable from only executable identity and duration.** Published ATPP additionally uses cell-tower location and nearby POI features, and initializes a general model from many users before per-user fine-tuning. A location-free temporal ablation inspired by ATPP/RMTPP is testable; calling that ablation “ATPP” would overstate what was reproduced.

“Confirmed-testable” below means that the model's required variables can be derived from Soul's allowed record and that its outputs have deterministic offline tests. It does **not** mean its accuracy on desktop foreground switching is already confirmed.

## Exact Soul observation model

For a completed foreground session \(i\), the allowed observation is:

\[
x_i=(a_i,t_i,d_i)
\]

where \(a_i\) is a lower-cased executable image name such as `code.exe`, \(t_i\) is the event/session start time, and \(d_i\) is foreground duration. No title, path, document, keys, pointer events, screen content, location, or third-person content is available.

Two qualifications matter:

1. Soul predicts the **next foreground executable/session**, not necessarily a process launch. Desktop alt-tab/return behavior differs from mobile “app opened” traces used by the cited papers.
2. If “only exe + duration” is interpreted literally as deleting \(t_i\), next-app identity models remain testable, but Hawkes/MTPP next-time prediction is not identifiable. Soul's existing event envelope has the session-start timestamp, so a local derived sequence can supply \(t_i\) without collecting another signal. Duration alone must not be mislabeled as an inter-arrival timestamp.

The prediction vocabulary must be executables observed locally. Soul does not enumerate installed programs, whereas SeqMF assumes a known set of apps installed on each device. An unseen executable therefore needs an explicit `unknown/no confident prediction` path rather than a fabricated score.

## Literature comparison

### 1. Count baselines: MFU, MRU, Markov, n-gram, APPM

An order-\(q\) Markov/n-gram predictor estimates

\[
P(a_{i+1}=b\mid a_{i-q+1:i})
\]

from prefix/follower counts. Order 0 is effectively Most Frequently Used (MFU); order 1 is a transition matrix or Sequential Rules model. Higher orders distinguish histories that end in the same app but arrived there through different workflows. Backoff/interpolation to shorter contexts is required for unseen prefixes.

Räsänen and Saarinen explicitly use fixed-order n-grams and mixed-order Markov chains as next-launched-application baselines, and describe the sparsity/limited-context tradeoff: [Sequence Prediction With Sparse Distributed Hyperdimensional Coding Applied to the Analysis of Mobile Phone Use Patterns](https://doi.org/10.1109/TNNLS.2015.2462721). A direct mobile transition analysis also treats per-device app transitions as Markov chains: [Gouin-Vallerand et al., “An analysis of the transitions between mobile application usages based on Markov chains”](https://ubicomp.org/ubicomp2014/proceedings/ubicomp_adjunct/programming-comp/p373-gouin-vallerand.pdf).

APPM is a useful variable-order version. It adapts Prediction by Partial Match to app sequences, incrementally counts follower apps at several prefix lengths, and weights prefix orders by their historical predictive accuracy. Its separate Time Till Usage (TTU) component learns an empirical conditional CDF of the next app's delay. The paper reports that adding location and time-of-day only marginally changed its mobile top-5 result, making its prefix-only form especially relevant to Soul's narrow data surface: [Parate et al., “Practical Prediction and Prefetch for Faster Access to Applications on Mobile Phones”](https://doi.org/10.1145/2493432.2493490) ([author PDF](https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/ubi1443-parate.pdf)).

#### Confirmed-testable variants

**MFU**

* Input: prior executable identities; optionally a rolling event/time window.
* Output: ranked executables and \(P(b)=\frac{c_b+\alpha}{N+\alpha K}\).
* Countability/explanation: “`code.exe` occurred 31 of the last 100 completed sessions.”
* Lock conflicts: none if counts and predictions stay local; duration is unused.
* Falsifier: if it cannot beat a deterministic random/tie policy on frequency-skew fixtures, the implementation is wrong. It is the minimum accuracy comparator, not the intended winner.

**MRU**

* Input: ordered executable identities.
* Output: most recently observed distinct executable(s).
* Countability/explanation: exact recency rank and elapsed session count.
* Lock conflicts: none.
* Falsifier: deterministic fixtures where the next app repeats a stable transition rather than recency should expose its weakness. If a complex model cannot beat MRU, reject the complexity.

**First-order Markov / local Sequential Rules**

* Input: adjacent pairs \(a_i\rightarrow a_{i+1}\).
* Output: ranked followers, smoothed transition probability, and support count.
* Countability/explanation: “After `outlook.exe`, `chrome.exe` followed 8/11 times.”
* Storage/compute: \(O(E)\) sparse transition counts for observed edges, not a dense \(K^2\) table.
* Lock conflicts: none; this is entirely per-device. Do not pool users.
* Falsifier: a held-out walk-forward sequence must beat MFU/MRU on transition-structured fixtures; probability rows must normalize and unseen states must back off rather than panic or silently choose lexicographically.

**Duration-conditioned first-order Markov (semi-Markov baseline)**

* Input: \((a_i,\operatorname{bucket}(d_i))\rightarrow a_{i+1}\).
* Output: follower distribution for the current executable and an explainable duration band.
* Countability/explanation: “After a short `chrome.exe` session, `code.exe` followed 6/8 times.”
* Storage/compute: \(O(BE)\) at most for \(B\) fixed, user-visible duration buckets.
* Lock conflicts: none. Buckets should remain stable and derived locally; they must not imply task/content semantics.
* Falsifier: construct fixtures where the same executable has different followers after short versus long sessions. Reject this feature if chronological log loss/HR@k is no better than the unconditioned transition model or if low-support buckets make calibration worse.

**Interpolated n-gram or APPM-style variable-order predictor**

* Input: the last \(q\) executable identities; APPM can update online and discount old counts.
* Output: a normalized top-k distribution, per-order support, and selected/interpolated contexts.
* Countability/explanation: “The suffix `excel.exe → outlook.exe` occurred 5 times; `teams.exe` followed 4 times; the order-1 backoff contributes 20%.”
* Storage/compute: bounded by retained distinct prefixes. Soul should cap \(q\), minimum support, and model bytes.
* Lock conflicts: none for prefix-only APPM. APPM's prefetching behavior is out of scope; prediction must not launch or preload apps.
* Falsifier: use an ambiguity fixture where `C` follows both `A` and `B`, but `A,C→D` and `B,C→E`. The higher-order predictor must beat first-order Markov after warm-up, back off correctly on unseen suffixes, remain bounded under many unique executables, and adapt after a deliberate regime change.

**Empirical TTU/delay CDF**

* Input: next-session start deltas \(\Delta_i=t_{i+1}-t_i\), grouped by next executable or transition; a simpler duration-only fallback can predict \(d_i\), but it is a different target.
* Output: median/quantiles or \(P(\Delta\leq h\mid a_{i+1}=b)\).
* Countability/explanation: “7/10 observed transitions to `teams.exe` started within 90 seconds.”
* Lock conflicts: none if the existing session timestamp is used. No prefetch or automatic execution follows from the estimate.
* Falsifier: compare with global median delay and last-duration baselines using MAE plus interval coverage. Reject per-app conditioning when support is too low or held-out error/calibration is worse.

These are the strongest v0.2 candidates because they learn online, need one user's trace, expose exact support, and can forget/rebuild counts when source events are forgotten.

### 2. Hawkes and other marked temporal point processes

A multivariate Hawkes process assigns each executable \(k\) an intensity such as

\[
\lambda_k(t)=\mu_k+\sum_{j:t_j<t}\alpha_{a_j,k}g_k(t-t_j).
\]

It jointly represents *which mark occurs* and *when*. Positive kernels model short-term excitation: a past foreground event temporarily raises another executable's intensity. The foundational reference is [Hawkes, “Spectra of some self-exciting and mutually exciting point processes”](https://doi.org/10.1093/biomet/58.1.83).

RMTPP replaces a hand-specified influence structure with an RNN history embedding and models next event time and mark jointly: [Du et al., “Recurrent Marked Temporal Point Processes: Embedding Event History to Vector”](https://doi.org/10.1145/2939672.2939875) ([paper PDF](https://www.kdd.org/kdd2016/papers/files/rpp1081-duA.pdf)). This is more expressive but less count-explainable and needs substantially more observations per parameter.

Recent controlled evaluation warns against assuming neural TPP superiority. It recommends separating time and mark likelihood, predictive accuracy, and calibration; simple Hawkes-style decoders can be competitive for mark prediction, while neural models can be overconfident: [Bosser and Ben Taieb, “On the Predictive Accuracy of Neural Temporal Point Process Models for Continuous-time Event Data”](https://arxiv.org/abs/2306.17066).

#### Confirmed-testable scope

**Regularized local Hawkes**

* Input: \((t_i,a_i)\); duration may be added only as a declared covariate or used to define session endpoints. Start timestamps are mandatory.
* Output: per-executable intensity/probability over a stated horizon, next-mark ranking, and a time quantile.
* Countability/explanation: baseline intensity, observed event count, and largest learned pairwise excitation terms can be shown, but they are fitted coefficients rather than direct frequencies.
* Single-machine feasibility: yes for a small observed vocabulary using sparse/low-rank or heavily regularized parameters. A dense \(K\times K\) influence matrix is unsafe for a long-tail desktop vocabulary.
* Lock conflicts: no new collection is needed. The word “excitation” must not be presented as a psychological cause.
* Falsifier: require better chronological mark log loss/HR@k than smoothed Markov and better time MAE/NLL than empirical TTU. Reject if fitted branching behavior is unstable, intensities explode, calibration fails, or coefficients change radically under small fixture perturbations.

**Small temporal-only RMTPP**

* Input: recent executable marks and log inter-start deltas; no title/location/content.
* Output: next-mark softmax and next-time distribution.
* Countability/explanation: parameter count, input history, top-k probabilities, and ablation results are countable, but the hidden state is not a user-legible reason. Explanations must therefore fall back to observable recent sessions and confidence, not claim a causal rationale.
* Single-machine feasibility: yes as a bounded offline research model; not yet justified for product inference. CPU feasibility is not the same as enough one-user data.
* Lock conflicts: possible conflict with explainability/correctability and evidence-before-storage. Do not persist learned “insights” as facts.
* Falsifier: paired walk-forward comparison against APPM/Markov; reject on no material gain, poor confidence calibration, seed instability, excessive warm-up, or resource-budget failure.

If timestamps are unavailable to the model, both time branches must be marked **not testable**. A mark-only RNN would no longer test an MTPP claim.

### 3. Federated SeqMF (arXiv:2303.04744)

Sayapin et al. formulate next-app prediction as sequence-aware matrix factorization. A user embedding \(p_u\) captures long-term preference; app embeddings \(Q\) are shared; a local transition matrix \(S_u\) adds sequential co-occurrence. Inference scores observed/installed candidates using long-term and recent-app components. User embeddings and history stay on device, while app embeddings are updated on a server from perturbed client gradients using the proposed QHarmony local-DP mechanism: [“Federated Privacy-preserving Collaborative Filtering for On-Device Next App Prediction”](https://arxiv.org/abs/2303.04744) ([published version](https://doi.org/10.1007/s11257-024-09395-0), [research code](https://github.com/grandient-daddy/Federated-Learning-with-Privacy)).

Important paper results qualify the headline:

* SeqMF uses multiple users and server-side shared app embeddings; that is the source of collaborative information.
* It predicts app identity, not next-event time; duration is unused.
* In the paper's static experiments, simple Sequential Rules variants outperform SeqMF on the reported datasets. The paper's main advantage is the dynamic federated privacy-utility tradeoff, not universal next-app accuracy.
* Candidate predictions are restricted to apps known to be installed. Soul only observes foreground executables and does not possess that inventory.

#### Soul status: not confirmed-testable as a v0.2 claim

* Required input: histories and installed-app sets from many users, plus client/server rounds. One local Soul history is insufficient to test collaborative filtering.
* Output: top-n relevance scores; model size, gradient messages, privacy budget \(\epsilon\), and HR/MRR/NDCG are countable.
* Conflicts: remote coordination and model-update egress violate the default local track before v0.4; a privacy mechanism does not itself grant permission to transmit. QHarmony also cannot make a single-user deployment federated.
* What one PC can test: a synthetic multi-client simulator can exercise matrix dimensions, update equations, QHarmony randomization, and deterministic limits. That is only a mechanical test and must use synthetic fixture identities, not real users.
* Falsifier for future authorized work: compare federated SeqMF against local SR/Markov with equal chronological histories. Require a gain that survives client dropout, unseen executables, privacy noise, and communication accounting. If simulated gains disappear without cross-user leakage, or local SR remains better, reject it.

The locally computable \(S_u\)/Sequential Rules component is already covered by the first-order Markov candidate and does not require adopting SeqMF.

### 4. ATPP

ATPP models app identity and open time jointly with a learned MTPP. It uses app embeddings, a GRU, attention over recent app events, and a spatial feature extractor derived from cell-tower location and 23 categories of nearby points of interest. The reported experiment trains a general model on many users, then fine-tunes each user's model. The paper compares against homogeneous Poisson, Hawkes, and an RMTPP-like RNN, reporting better next-app and time metrics on its own mobile dataset: [Yang et al., “ATPP: A Mobile App Prediction System Based on Deep Marked Temporal Point Processes”](https://doi.org/10.1109/DCOSS52077.2021.00028) ([extended article](https://doi.org/10.1145/3582555), [author PDF](https://sites.ucmerced.edu/files/wdu/files/2021dcoss_-_atpp.pdf), [released code](https://github.com/kangyang73/atpp_dcoss21)).

#### Soul status

**Full ATPP: not confirmed-testable.**

* Missing input: Soul deliberately has no location, cell tower, POI vector, mobile app inventory, or multi-user pretraining corpus.
* Portability: the released project documents Ubuntu 16.04, Python ≥3.5, and TensorFlow 1.x. Its presence on GitHub does not establish a supported one-Windows-machine reproduction.
* Conflict: adding location or cloud/general-user training solely to reproduce ATPP would violate the v0.2 data and egress locks.

**Temporal-only ATPP/RMTPP ablation: confirmed-testable as a separate model.**

* Input: recent \((a_i,t_i)\), optionally \(d_i\), with location branch removed.
* Output: next executable distribution and next-start-time distribution.
* Countability: HR@k, MRR, mark/time NLL, MAE, calibration, parameter count, memory, and latency. Attention weights are inspectable but are not proof of causation.
* Naming: report it as “temporal-only GRU MTPP” or “ATPP-inspired ablation,” not as reproduced ATPP.
* Falsifier: it must beat the same-history APPM/Markov and empirical TTU baselines on held-out local fixtures, remain calibrated, and be stable across seeds. Also compare a GRU without MTPP loss to test whether continuous-time modeling—not merely model capacity—causes any gain.

ATPP's published MAE evaluation ignores whether the predicted app ID is correct. Soul should additionally report a joint decision metric, for example the time error only for a correctly ranked top-k mark plus miss rate, so excellent timing for the wrong executable cannot look successful.

## One-Windows-machine test matrix

| Candidate | Exact exe sequence | Duration | Existing start timestamp | Multiple real users/server | v0.2 result |
|---|---:|---:|---:|---:|---|
| MFU / MRU | yes | no | no | no | **confirmed-testable** |
| First-order Markov / SR-od | yes | no | no | no | **confirmed-testable** |
| Duration-conditioned Markov | yes | yes | no | no | **confirmed-testable** |
| Variable-order n-gram / APPM | yes | no | no | no | **confirmed-testable** |
| Empirical TTU CDF | yes | optional | yes | no | **confirmed-testable** |
| Regularized local Hawkes | yes | optional | yes | no | **confirmed-testable research comparator** |
| Temporal-only RMTPP / ATPP-inspired ablation | yes | optional | yes | no | **confirmed-testable research comparator** |
| Full ATPP | yes | optional | yes | general-model users + location | **not confirmed-testable** |
| Local-only SeqMF ablation | yes | no | no | no, but loses federated claim | mechanically testable, low v0.2 relevance |
| Federated SeqMF + QHarmony | yes | no | no | yes | **not a v0.2 one-machine efficacy test** |

## Required later falsification protocol

### Data provenance: FakeForegroundSource only

**LSApp is not Soul data.** It is an external mobile dataset with its own collection process, population, event types, and app vocabulary. Its documented columns include user/session identifiers, timestamp, app name, and interaction type: [LSApp dataset record](https://doi.org/10.21227/w17r-xx75) and [dataset repository](https://github.com/aliannejadi/LSApp). It may explain prior literature, but it must not be used as acceptance evidence for Soul's Windows collector or predictor.

All later Soul tests must create deterministic traces through `FakeForegroundSource` fixtures and the real collection/session path. They must not hand-insert rows that merely resemble collector output. Fixture executable names should be synthetic (`alpha.exe`, `beta.exe`, …), and fixture bodies must contain only executable identity and duration with the normal event timestamp.

Minimum fixture families:

1. **Frequency skew:** no transition signal; verifies MFU and prevents a complex model from claiming trivial popularity.
2. **First-order workflow:** `alpha→beta`, `gamma→delta`; verifies transition counts.
3. **Higher-order ambiguity:** both histories end in the same executable but have different next executables; verifies n-gram/APPM value.
4. **Duration split:** short and long sessions of one executable have different followers; verifies the semi-Markov baseline without content.
5. **Burst versus pause:** equal mark order with different inter-event timing; verifies that Hawkes/MTPP uses time rather than only marks.
6. **Concept drift/new executable:** the generating pattern changes midstream and introduces an unseen `.exe`; measures adaptation and abstention.
7. **Collection edge cases:** `None`/lock gaps, source error, repeated same executable, zero duration from clock rollback, stop, and consent revocation. Predictions must not bridge a gap as if it were a known direct transition unless that policy is explicit.

### Evaluation

Use chronological prequential evaluation: at each event, predict using only prior events, score the next event, then update. Also report a frozen chronological train/validation/test split for models needing tuning. Never randomly shuffle transitions, because that leaks future regimes into training.

For next executable:

* top-1 accuracy and HR@3;
* MRR@3;
* macro recall or per-executable recall so a dominant app cannot hide failures;
* log loss/Brier score and reliability bins when probabilities are emitted;
* abstention/coverage and unseen-executable count;
* warm-up events before the first supported prediction.

For next time:

* median absolute error and MAE;
* time NLL and empirical interval coverage/calibration;
* mark-conditioned and joint mark+time results;
* separate known-gap versus lock/unknown-gap cases.

For local feasibility:

* model bytes, peak memory, update latency, prediction latency, and deterministic rebuild time;
* number of executable states, observed edges/prefixes, trainable parameters, and minimum support;
* identical results after export/rebuild from the same non-forgotten fixture events;
* deletion test: forgetting source events must remove their contribution rather than leave an untraceable learned fact.

### Decision rule

No model wins by paper prestige. Keep multiple models “confirmed-testable,” then reject any candidate that:

1. does not outperform the simpler predecessor on the fixture family designed to express its claimed advantage;
2. loses to MFU/MRU/Markov on cumulative chronological metrics without a compensating, predefined benefit;
3. needs a forbidden field or cross-device data;
4. cannot expose support/confidence in user-correctable language;
5. is unstable across seeds or small perturbations;
6. exceeds a fixed local resource bound; or
7. leaves learned state after the evidence that produced it is forgotten.

## Recommendation for Soul v0.2

Test, in order, without choosing a winner in advance:

1. MFU and MRU controls.
2. Smoothed first-order Markov/SR-od.
3. Duration-conditioned first-order Markov.
4. Bounded interpolated n-gram/APPM.
5. Empirical TTU if “when” is in scope.
6. Regularized Hawkes and a small temporal-only RMTPP only as research challengers.

Defer federated SeqMF to an explicitly authorized multi-device/cloud phase no earlier than v0.4. Treat full ATPP as incompatible with the current input lock; only its location-free temporal ablation belongs in the local test harness.
