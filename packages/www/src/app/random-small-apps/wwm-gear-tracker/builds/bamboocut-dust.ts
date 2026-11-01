// 破竹尘 (Bamboocut-Dust): 魂伞 + 绳镖, a 55.5 s rotation. 结算 rows settle
// a tenth of the damage of the rows they reference.

import { chances, elementOf, expectedHit, isOne, targetOf, type BuildDefinition } from "./common";
import { PRESET, ROTATION, SKILLS, type Input, type Skill } from "./bamboocut-dust.data";

const ROTATION_SECONDS = 55.5;
const AFF_DAMAGE = 1.35;
const FIXED_BONUS = 0.0725;

/** Hits on 气竭 or 断石 targets, which 断石之构 boosts. */
const onBrokenTarget = (name: string) => name.includes("气竭") || name.includes("断石");

function skillDamage(
  name: string,
  skill: Skill,
  input: Input,
  yishuigePenetration: number,
): number {
  const s = input.panelAttributes;
  const { martialArts: arts } = input;
  const defense = targetOf(input.targetType).defense * (arts.suohenniannian ? 0.94 : 1);
  const element = elementOf(s, skill.element || "破竹");
  const armor = input.armorSet === "飞隼" ? 1.1 : input.armorSet === "撼天" ? 1.05 : 1;
  const min =
    s.physicalAttackMin * (1 + (skill.extraMinPhysical || 0)) * armor -
    defense +
    (arts.ganglushaobing ? 140 : 0);
  const max =
    Math.max(s.physicalAttackMax * (1 + (skill.extraMaxPhysical || 0)), min + defense) * armor -
    defense +
    (arts.ganglushaobing ? 280 : 0);
  const avg = (min + max) / 2;
  let penetration = skill.extraPenetration || 0;
  if (arts.suohenniannian) penetration += 10;
  if (arts.yishuige) penetration += yishuigePenetration;
  if (arts.duanshizhigou && onBrokenTarget(name)) penetration += 25;
  const pen = (s.physicalPenetration + penetration) / 200;
  let damageBonus = s.physicalDamageBonus || 0;
  if (arts.suohenniannian) damageBonus += 0.025;
  if (skill.special === "回旋伞") damageBonus += 0.15;
  const corr = skill.correction || 1;

  let crit: number;
  let aff: number;
  if (skill.forceCrit) {
    crit = 1;
    aff = 0;
  } else {
    let direct = s.directCriticalRate;
    if (arts.duanshizhigou) direct += 0.041;
    if (arts.yishuige) direct += 0.046;
    crit = Math.min(s.criticalRate + (skill.extraCritRate || 0), 0.8) + Math.min(direct, 0.2);
    aff =
      Math.min(s.affinityRate + (skill.extraAffRate || 0), 0.4) +
      Math.min(s.directAffinityRate, 0.1);
  }
  const p = chances(crit, aff, Math.min(s.precisionRate, 1));
  const critBonus = 0.5 + (input.armorSet === "时雨" ? 0.1 : 0);
  let critDamage = skill.extraCritDamage || 0;
  if (arts.duanshizhigou && onBrokenTarget(name)) critDamage += 0.25;
  const multipliers = {
    glance: 1,
    crit: 1 + critBonus + critDamage,
    aff: AFF_DAMAGE + (skill.extraAffDamage || 0),
  };

  const physical = (1 + pen) * (1 + damageBonus) * corr;
  let total = expectedHit(p, min, avg, max, skill.multiplier * physical, multipliers);
  const fixed = skill.fixedDamage * (1 + (skill.skillType === "武器" ? FIXED_BONUS : 0));
  total += expectedHit(p, fixed, fixed, fixed, physical, multipliers);

  const em = skill.elementMultiplier;
  if (em > 0 && element.min > 0) {
    const base = skill.skillType === "武器" ? 150.7 * 1.0725 : 0;
    const eMin = element.min + base;
    const eMax = Math.max(element.max + base, eMin);
    const scale = em * (1 + element.penetration / 200) * (1 + element.damageBonus);
    total += expectedHit(p, eMin, (eMin + eMax) / 2, eMax, scale, multipliers);
  }
  if ((s.wuxiangAttackMin > 0 || s.wuxiangAttackMax > 0) && em > 0) {
    const scale =
      em * (1 + (element.penetration + s.wuxiangPenetration) / 200) * (1 + element.damageBonus);
    const { wuxiangAttackMin: wMin, wuxiangAttackMax: wMax } = s;
    total += expectedHit(p, wMin, (wMin + wMax) / 2, wMax, scale, multipliers);
  }
  return total;
}

function dps(input: Input): number {
  const { martialArts: arts, weaponBonus: wb } = input;
  let general = targetOf(input.targetType).generalBonus;
  if (arts.yishuige) general += 0.05;
  if (arts.zhengrengui) general += 0.08;
  if (arts.tiangong === "火") general += 0.015;
  else if (arts.tiangong === "毒") general += 0.01;
  general += wb.boss;

  let total = 0;
  const byRow = new Map<number, number>();
  const settlements: (typeof ROTATION)[number][] = [];
  for (const row of ROTATION) {
    if (row.name === "结算") {
      settlements.push(row);
      continue;
    }
    const skill = SKILLS[row.name];
    if (skill == null || row.count === 0) {
      byRow.set(row.row, 0);
      continue;
    }
    const damage = skillDamage(row.name, skill, input, row.yishuigePenetration ?? 10);
    let bonus = general + (skill.extraBonus || 0);
    if (isOne(skill.isCharge)) bonus += input.generalBonus.charge;
    if (skill.weaponType === "伞") bonus += wb.umbrella + wb.allWeapon;
    else if (skill.weaponType === "绳标") bonus += wb.ropeDart + wb.allWeapon;
    else if (
      skill.weaponType === "N/A" ||
      skill.skillType === "奇术" ||
      skill.skillType === "心法"
    ) {
      bonus += wb.na;
    }
    if (arts.yishuige && row.yishuigePenalty) bonus -= row.yishuigePenalty;
    if (input.chenghuantie && row.chenghuantieBonus) bonus += row.chenghuantieBonus;
    const attuned = skill.weaponType === "伞" && row.name.includes("Q") ? wb.umbrellaQDingyin : 0;
    const rowDamage = damage * (1 + bonus) * (skill.correction || 1) * (1 + attuned) * row.count;
    total += rowDamage;
    byRow.set(row.row, rowDamage);
  }
  for (const { jiesuanRefs: refs } of settlements) {
    let settled = 0;
    // Inclusive [from, to] row ranges.
    for (const [from = 0, to = -1] of refs?.ranges ?? []) {
      for (let r = from; r <= to; r++) settled += byRow.get(r) ?? 0;
    }
    for (const r of refs?.extra_rows ?? []) settled += byRow.get(r) ?? 0;
    total += settled / 10;
  }
  return total / ROTATION_SECONDS;
}

export const BAMBOOCUT_DUST: BuildDefinition<Input> = {
  id: "bamboocut-dust",
  name: "破竹尘",
  baselineKey: "pozhuchen",
  preset: PRESET,
  dps,
  mainWeapon: "umbrella",
  rawRates: false,
};
