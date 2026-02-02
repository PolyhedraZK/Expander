# HyperKZG Batch Opening 内存优化分析

## 问题背景

`multiple_points_batch_open_impl` 在处理大量多项式时，内存消耗爆炸。特别是在 "Building tilde g_i(b)" 阶段及其后续的 sumcheck 阶段，内存达到峰值。

## 性能热点（实测）

以下三步开销最大，**重点关注**：

```
* Building tilde eqs
* Building tilde eqs 3.42449ms
* Sumcheck merging points        <-- 最高开销！
* Sumcheck merging points 40.561453ms
* Building g'(X)
* Building g'(X) 8.401878ms
```

**Sumcheck merging points** 是最大的瓶颈，包含：
1. `sumcheck_poly.add_pair(tilde_g.clone(), tilde_eq)` - 克隆 tilde_gs
2. `SumCheck::prove()` - 内部又会克隆一次（已优化为移动）

## 内存消耗数据流分析

假设有 **k** 个多项式，每个大小约 **2^n** 个字段元素：

```
               ┌──────────────────────────────────────────────────────────────────┐
               │                    prover_merge_points                           │
               ├──────────────────────────────────────────────────────────────────┤
               │                                                                   │
  polys ─────► │  "Building tilde g_i(b)"                                         │
  (输入)        │     └─► tilde_gs: Vec<MultiLinearPoly>  [k × 2^n] ◄───────────┐  │
               │                                                    │          │  │
               │  "Building tilde eqs"                              │          │  │
               │     └─► tilde_eqs: Vec<MultiLinearPoly> [k × 2^n]  │          │  │
               │                        │                           │          │  │
               │                        │ into_iter() (移动)        │ clone()  │  │
               │                        ▼                           │          │  │
               │  "Sumcheck merging"    sumcheck_poly ──────────────┘          │  │
               │     │                     │                                   │  │
               │     │                     │ .clone() in prover_init           │  │
               │     │                     ▼                                   │  │
               │     │           IOPProverState.mle_list [2k × 2^n]            │  │
               │     │                                                         │  │
               │  "Building g'(X)" ◄──────────────────────────────────────────┘  │
               │     └─► g_prime_evals [2^n]                                      │
               └──────────────────────────────────────────────────────────────────┘
```

## 关键问题点

### 问题 1: `tilde_g.clone()`

**位置**: `batching.rs:81`

```rust
for (tilde_g, tilde_eq) in tilde_gs.iter().zip(tilde_eqs.into_iter()) {
    sumcheck_poly.add_pair(tilde_g.clone(), tilde_eq);  // <-- 克隆！
}
```

`tilde_gs` 被**完整克隆**一份到 `sumcheck_poly`，因为后面 "Building g'(X)" 还要用 `tilde_gs`。

### 问题 2: `polynomials.clone()`

**位置**: `sumcheck/src/sumcheck_generic/prover.rs:18`

```rust
pub fn prover_init(polynomials: &SumOfProductsPoly<F>) -> Self {
    // ...
    mle_list: polynomials.clone(),  // <-- 又克隆了整个 sumcheck_poly！
    // ...
}
```

`SumCheck::prove` 接受的是引用，然后内部又做了一次完整克隆。

## 内存峰值估算

在 SumCheck 开始时，内存中同时存在：

| 数据结构 | 大小 |
|---------|------|
| `polys` (输入，不能释放) | k × 2^n |
| `tilde_gs` (需要给 Building g'(X) 用) | k × 2^n |
| `sumcheck_poly` (tilde_gs 的克隆 + tilde_eqs) | 2k × 2^n |
| `IOPProverState.mle_list` (sumcheck_poly 的克隆) | 2k × 2^n |
| **峰值总计** | **~6k × 2^n** |

---

## 优化方案

### 优化 1: 消除 `prover_init` 中的克隆（推荐，立竿见影）

**难度**: 低
**收益**: 减少约 2k × 2^n 内存（约 1/3 峰值内存）

**当前代码** (`sumcheck_generic.rs:96-102`):
```rust
pub fn prove(
    poly_list: &SumOfProductsPoly<F>,  // 引用
    transcript: &mut impl Transcript,
) -> IOPProof<F> {
    let mut prover_state = IOPProverState::prover_init(poly_list);  // 内部克隆
```

**建议修改**:
```rust
pub fn prove(
    poly_list: SumOfProductsPoly<F>,  // 接受所有权
    transcript: &mut impl Transcript,
) -> IOPProof<F> {
    let mut prover_state = IOPProverState::prover_init_owned(poly_list);  // 移动，无克隆
```

同时修改 `prover.rs`:
```rust
pub fn prover_init_owned(polynomials: SumOfProductsPoly<F>) -> Self {
    let num_vars = polynomials.num_vars();
    let init_sum_of_vals = polynomials
        .f_and_g_pairs
        .par_iter()
        .map(|(f, g)| {
            f.coeffs.iter().zip(g.coeffs.iter()).map(|(&f, &g)| f * g).sum::<F>()
        })
        .collect();
    let eq_prefix = vec![F::one(); polynomials.f_and_g_pairs.len()];

    Self {
        challenges: Vec::with_capacity(num_vars),
        round: 0,
        init_num_vars: num_vars,
        mle_list: polynomials,  // 直接移动，无克隆
        init_sum_of_vals,
        eq_prefix,
    }
}
```

### 优化 2: 重构算法顺序避免保留 `tilde_gs`

**难度**: 中
**收益**: 减少约 k × 2^n 内存

观察：
1. "Building g'(X)" 需要 `tilde_gs` 和 `a2`（sumcheck 的输出点）
2. 在 sumcheck 过程中，`tilde_gs` 的系数会被 `fix_top_variable` 逐步缩减
3. Sumcheck 结束时，每个 `tilde_g` 只剩 1 个系数

**思路**: 利用 sumcheck 后的缩减值重建 `g_prime`，而不是保留原始 `tilde_gs`。

### 优化 3: 流式/惰性计算 `tilde_gs`

**难度**: 中
**收益**: 峰值内存可控

**当前**：一次性并行计算所有 `tilde_gs`
```rust
let tilde_gs = polys.par_iter().enumerate().map(...).collect::<Vec<_>>();
```

**建议**：分批处理
```rust
const BATCH_SIZE: usize = 64;  // 根据可用内存调整

for batch_start in (0..k).step_by(BATCH_SIZE) {
    let batch_end = (batch_start + BATCH_SIZE).min(k);
    let batch_polys = &polys[batch_start..batch_end];

    // 处理这个 batch
    let batch_tilde_gs = batch_polys.par_iter().enumerate().map(...).collect();

    // 累加到 sumcheck_poly
    for (tilde_g, tilde_eq) in batch_tilde_gs.into_iter().zip(...) {
        sumcheck_poly.add_pair(tilde_g, tilde_eq);
    }
    // batch_tilde_gs 在这里被释放
}
```

### 优化 4: 使用引用而非克隆（需要较大重构）

**难度**: 高
**收益**: 最优内存效率

修改 `SumOfProductsPoly` 使用引用：
```rust
pub struct SumOfProductsPoly<'a, F: Field> {
    pub f_and_g_pairs: Vec<(&'a MultiLinearPoly<F>, MultiLinearPoly<F>)>,
}
```

这样 `tilde_gs` 不需要克隆，sumcheck 可以直接引用它们。

**注意**: 这需要修改整个 sumcheck 模块的生命周期标注。

### 优化 5: 内存映射/懒加载（适用于极端情况）

**难度**: 高
**收益**: 可处理超大规模数据

如果多项式太大无法全部放入内存，可以考虑：
- 使用 memory-mapped files
- 实现按需加载的多项式存储
- 流式处理 sumcheck

---

## 实施优先级建议

| 优先级 | 优化方案 | 难度 | 收益 |
|-------|---------|------|------|
| 1 | 优化 1: 消除 prover_init 克隆 | 低 | 高 (~33%) |
| 2 | 优化 2: 重构避免保留 tilde_gs | 中 | 中 (~17%) |
| 3 | 优化 3: 分批处理 | 中 | 可控峰值 |
| 4 | 优化 4: 引用重构 | 高 | 最优 |

---

## 调试/验证方法

### 内存监控已集成

已添加详细的内存监控到以下文件：
- `utils/src/memory_profiler.rs` - 内存监控工具模块
- `poly_commit/src/kzg/uni_kzg/hyper_kzg.rs` - `multiple_points_batch_open_impl`
- `poly_commit/src/batching.rs` - `prover_merge_points`
- `sumcheck/src/sumcheck_generic.rs` - `SumCheck::prove`
- `sumcheck/src/sumcheck_generic/prover.rs` - `IOPProverState::prover_init`

### 启用内存监控

使用 `mem-profile` feature 编译并运行：

```bash
# 检查编译
cargo check -p poly_commit --features mem-profile

# 运行测试（带内存监控）
cargo test -p poly_commit --features mem-profile -- --nocapture

# 运行特定基准测试
cargo bench -p poly_commit --features mem-profile
```

### 监控输出示例

启用 `mem-profile` 后，输出格式如下：

```
======================================================================
[MEM BATCH_OPEN] Starting batch opening
[MEM BATCH_OPEN] Input: 1024 polys, 16777216 total elements, field_size=32 bytes
[MEM BATCH_OPEN] Estimated input size: 512.00 MB
[MEM BATCH_OPEN] Initial RSS: 1024.00 MB
======================================================================
  [MEM START] multiple_points_batch_open_impl | RSS: 1024.00 MB | Delta: +0.00 MB
    [MEM START] prover_merge_points | RSS: 1024.00 MB | Delta: +0.00 MB
      [MEM CHECKPOINT] prover_merge_points :: before building tilde_gs | RSS: 1024.00 MB
      [MEM DETAIL] tilde_gs actual size: 512.00 MB (1024 polys)
      [MEM CHECKPOINT] prover_merge_points :: after building tilde_gs | RSS: 1536.00 MB | +512.00 MB
      [MEM DETAIL] tilde_eqs actual size: 512.00 MB (1024 polys)
      [MEM CHECKPOINT] prover_merge_points :: after building tilde_eqs | RSS: 2048.00 MB | +1024.00 MB
      [MEM DETAIL] Adding pair 1/1024 to sumcheck_poly | RSS: 2049.00 MB
      [MEM DETAIL] Adding pair 100/1024 to sumcheck_poly | RSS: 2150.00 MB
      ...
      [MEM DETAIL] sumcheck_poly actual size: 1024.00 MB (1024 pairs)
      [MEM CHECKPOINT] prover_merge_points :: before SumCheck::prove | RSS: 3072.00 MB
        [MEM START] SumCheck::prove | RSS: 3072.00 MB
          [MEM PROVER_INIT] About to clone polynomials (THIS IS THE BIG ALLOCATION)...
          [MEM CHECKPOINT] IOPProverState::prover_init :: after clone | RSS: 4096.00 MB | +1024.00 MB
        [MEM END] SumCheck::prove | RSS: 4096.00 MB | +1024.00 MB
      [MEM CHECKPOINT] prover_merge_points :: after SumCheck::prove | RSS: 4096.00 MB
      [MEM DETAIL] Dropping sumcheck_poly...
      [MEM CHECKPOINT] prover_merge_points :: after dropping sumcheck_poly | RSS: 3072.00 MB
    [MEM END] prover_merge_points | RSS: 2560.00 MB | +1536.00 MB
  [MEM END] multiple_points_batch_open_impl | RSS: 2560.00 MB
======================================================================
[MEM BATCH_OPEN] Final RSS: 2560.00 MB | Total delta: +1536.00 MB
======================================================================
```

### 监控点列表

| 位置 | 监控点 | 说明 |
|------|--------|------|
| `multiple_points_batch_open_impl` | START/END | 整个 batch opening 的入口/出口 |
| `multiple_points_batch_open_impl` | before/after eval all polys | 多项式求值 |
| `multiple_points_batch_open_impl` | before/after prover_merge_points | merge points 调用 |
| `multiple_points_batch_open_impl` | before/after kzg_open | KZG open 调用 |
| `prover_merge_points` | after params calculation | 参数计算后 |
| `prover_merge_points` | before/after building tilde_gs | tilde_g 构建 |
| `prover_merge_points` | before/after building tilde_eqs | tilde_eq 构建 |
| `prover_merge_points` | before/after sumcheck_poly construction | sumcheck_poly 构建 |
| `prover_merge_points` | before/after SumCheck::prove | sumcheck 证明 |
| `prover_merge_points` | after dropping sumcheck_poly | 释放 sumcheck_poly |
| `prover_merge_points` | after dropping tilde_gs | 释放 tilde_gs |
| `SumCheck::prove` | before/after prover_init | prover 初始化 |
| `SumCheck::prove` | ROUND x/n | 每轮 sumcheck |
| `IOPProverState::prover_init` | before/after clone | **关键克隆点** |

---

## 相关文件

- `poly_commit/src/batching.rs` - prover_merge_points 函数
- `poly_commit/src/kzg/uni_kzg/hyper_kzg.rs` - multiple_points_batch_open_impl 函数
- `sumcheck/src/sumcheck_generic.rs` - SumCheck::prove 函数
- `sumcheck/src/sumcheck_generic/prover.rs` - IOPProverState::prover_init 函数
- `arith/polynomials/src/sum_of_products.rs` - SumOfProductsPoly 结构
- `arith/polynomials/src/mle.rs` - MultiLinearPoly 结构
