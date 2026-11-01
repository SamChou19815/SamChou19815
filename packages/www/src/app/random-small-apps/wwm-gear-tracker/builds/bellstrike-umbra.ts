// 鸣金影 (Bellstrike-Umbra): 积矩九剑 + 九曲惊神枪, a 57 s rotation. Unlike the
// other calculators, its panel keeps 精准/会心/会意 as raw in-game percentages and
// applies the target's judgment resistance itself.

import {
  chances,
  elementOf,
  ELEMENTS,
  expectedHit,
  targetOf,
  type BuildDefinition,
} from "./common";
import { PRESET, ROTATION, SKILLS, type Input, type Skill } from "./bellstrike-umbra.data";

const ROTATION_SECONDS = 57;
const CRIT_DAMAGE = 1.5;
const BELLSTRIKE_BONUS = 1 + 0.118;
const FIXED_BONUS = 0.0725;
// 150.7 × (1 + 7.18%): 武器 skills' flat 属攻.
const WEAPON_ELEMENT_BASE = 161.5185;
// The weapon bonuses the rotation table's jingshenBonus column already includes.
const TABLE_WEAPON_BONUS = { sword: 0.074, spear: 0, allWeapon: 0, na: 0 };

const isHigh = (level: string) => level === "五重" || level === "六重";

/** 断石之构 on skills that hit 断石/气竭 (or 崩解) targets. */
function breakBonus(name: string, level: string): { critDamage: number; penetration: number } {
  const broken = name.includes("断石") || name.includes("气竭");
  const collapse = name.includes("蹭崩解") || name.includes("崩解迷踪");
  if (broken) {
    if (level === "六重" || level === "四重") return { critDamage: 0.25, penetration: 25 };
    if (level === "三重") return { critDamage: 0.15, penetration: 15 };
    return { critDamage: 0, penetration: 0 };
  }
  if (collapse && level === "六重") return { critDamage: 0.25, penetration: 25 };
  return { critDamage: 0, penetration: 0 };
}

function generalBonus(input: Input): number {
  const { martialArts: arts } = input;
  let bonus = targetOf(input.targetType).generalBonus;
  if (isHigh(arts.yishuige)) bonus += 0.05;
  if (arts.zhenrengui) bonus += 0.08;
  if (arts.tiangong === "火") bonus += 0.015;
  else if (arts.tiangong === "毒") bonus += 0.01;
  switch (arts.zhulangxinjing) {
    case "六重":
      bonus += 0.3;
      break;
    case "五重":
    case "四重":
      bonus += 0.25;
      break;
    default:
      bonus += 0.1;
  }
  return bonus + input.weaponBonus.boss;
}

function skillDamage(
  name: string,
  skill: Skill,
  input: Input,
  penetration: number,
  defenseBonus: boolean,
): number {
  const s = input.panelAttributes;
  const { martialArts: arts } = input;
  const target = targetOf(input.targetType);
  const resistance = target.judgmentResistance;
  const armor = input.armorSet === "飞隼" ? 1.1 : 1;
  const extra = defenseBonus ? 0.05 * target.defense : 0;
  const min =
    s.physicalAttackMin * (1 + skill.extraMinPhysical) * armor -
    target.defense +
    (arts.ganglushaobing ? 140 : 0) +
    extra;
  const max =
    Math.max(s.physicalAttackMax * (1 + skill.extraMaxPhysical) * armor, min + target.defense) -
    target.defense +
    (arts.ganglushaobing ? 280 : 0) +
    extra;
  const avg = (min + max) / 2;
  const corr = skill.correction || 1;
  // The table's extraAffRate / extraCritRate columns are read crossed, as the site does.
  const crit =
    Math.min((s.criticalRate / 100 + skill.extraAffRate) / resistance, 0.8) +
    Math.min(s.directCriticalRate / 100, 0.2);
  const frozen = isHigh(arts.ningshengzhang) ? 0.03 : 0;
  const aff =
    Math.min((s.affinityRate / 100 + frozen + skill.extraCritRate) / resistance, 0.4) +
    Math.min(s.directAffinityRate / 100, 0.1);
  const precision = Math.min(0.65 + (s.precisionRate / 100 - 0.65) / resistance, 1);
  const p = chances(crit, aff, precision);
  let affBonus = 0.35;
  if (isHigh(arts.zhulangxinjing)) affBonus += 0.052;
  if (isHigh(arts.ningshengzhang)) affBonus += 0.1;
  const multipliers = {
    glance: 1,
    crit: CRIT_DAMAGE + skill.extraCritDamage + breakBonus(name, arts.duanshizhigou).critDamage,
    aff: 1 + affBonus + skill.extraAffDamage,
  };

  let total = expectedHit(
    p,
    min,
    avg,
    max,
    skill.multiplier * (1 + penetration) * corr,
    multipliers,
  );
  const fixed = skill.fixedDamage * (1 + (skill.skillType === "武器" ? FIXED_BONUS : 0));
  total += expectedHit(p, fixed, fixed, fixed, (1 + penetration) * corr, multipliers);

  const em = skill.elementMultiplier;
  if ((ELEMENTS as readonly string[]).includes(skill.element) && em > 0) {
    const element = elementOf(s, skill.element);
    const base = skill.skillType === "武器" ? WEAPON_ELEMENT_BASE : 0;
    const eMin = element.min + base;
    const eMax = Math.max(element.max + base, eMin);
    const scale = em * (1 + (skill.extraElementPen + element.penetration) / 200) * BELLSTRIKE_BONUS;
    total += expectedHit(p, eMin, (eMin + eMax) / 2, eMax, scale, multipliers);
  }
  if ((s.wuxiangAttackMin > 0 || s.wuxiangAttackMax > 0) && em > 0) {
    const scale = em * (1 + elementOf(s, skill.element).penetration / 200) * BELLSTRIKE_BONUS;
    const { wuxiangAttackMin: wMin, wuxiangAttackMax: wMax } = s;
    total += expectedHit(p, wMin, (wMin + wMax) / 2, wMax, scale, multipliers);
  }
  return total;
}

function dps(input: Input): number {
  const { martialArts: arts, weaponBonus: wb } = input;
  const general = generalBonus(input);
  let total = 0;
  for (const row of ROTATION) {
    const skill = SKILLS[row.name];
    if (skill == null) continue;
    // The table's penetration is final, plus 断石之构's.
    const penetration =
      row.penetration + breakBonus(row.name, arts.duanshizhigou).penetration / 200;
    const damage = skillDamage(row.name, skill, input, penetration, row.hasDefenseBonus || false);
    let bonus = general + (skill.extraBonus || 0);
    // The table's weapon bonus is swapped for the panel's.
    let weapon = row.jingshenBonus || 0;
    const t = TABLE_WEAPON_BONUS;
    if (skill.weaponType === "剑")
      weapon = weapon - t.sword - t.allWeapon + (wb.sword + wb.allWeapon);
    else if (skill.weaponType === "枪")
      weapon = weapon - t.spear - t.allWeapon + (wb.spear + wb.allWeapon);
    else if (
      skill.weaponType === "N/A" ||
      skill.skillType === "奇术" ||
      skill.skillType === "心法"
    ) {
      weapon = weapon - t.na + wb.na;
    }
    bonus += weapon;
    if (isHigh(arts.yishuige) && row.yishuigePenalty) bonus -= row.yishuigePenalty;
    if (arts.yishang && row.hasYishang) bonus += 0.08;
    if (arts.chenghuantie && row.hasChenghuantie) bonus += 0.2;
    bonus += row.fixedBonus || 0;
    const bleed =
      row.name.includes("流血") || row.name.includes("血爆") ? input.tuningBonus.bleed : 0;
    total +=
      damage *
      (1 + bonus) *
      (row.fMultiplier || 1) *
      (skill.correction || 1) *
      (1 + bleed) *
      row.count;
  }
  return total / ROTATION_SECONDS;
}

export const BELLSTRIKE_UMBRA: BuildDefinition<Input> = {
  id: "bellstrike-umbra",
  name: "鸣金影",
  baselineKey: "mingjinying",
  preset: PRESET,
  dps,
  mainWeapon: "sword",
  rawRates: true,
};
