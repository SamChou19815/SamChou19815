// 牵丝玉 (Silkbind-Jade): 九重春色 (伞) + 青山执笔 (扇), an 18.2 s rotation.

import { chances, elementOf, expectedHit, isOne, targetOf, type BuildDefinition } from "./common";
import { PRESET, ROTATION, SKILLS, type Input, type Skill } from "./silkbind-jade.data";

const ROTATION_SECONDS = 18.2;
const AFF_DAMAGE = 1.35;
const FIXED_BONUS = 0.0725;

function skillDamage(name: string, skill: Skill, input: Input): number {
  const s = input.panelAttributes;
  const { martialArts: arts } = input;
  const defense = targetOf(input.targetType).defense;
  // 易水歌 and 极乐泣血 hit with whichever 属攻 is the main one.
  const element = elementOf(
    s,
    name === "易水歌6重" || name === "极乐泣血" ? input.primaryAttribute : skill.element,
  );
  const attack = 1 + (skill.extraPhysicalBonus || 0) + (input.armorSet === "飞隼" ? 0.1 : 0);
  const min =
    s.physicalAttackMin * (1 + (skill.extraMinPhysical || 0)) * attack -
    defense +
    (arts.ganglushaobing ? 120 : 0);
  const max =
    Math.max(s.physicalAttackMax * (1 + (skill.extraMaxPhysical || 0)), min + defense) * attack -
    defense +
    (arts.ganglushaobing ? 240 : 0);
  const avg = (min + max) / 2;
  const pen =
    (s.physicalPenetration +
      (skill.extraPenetration || 0) +
      (arts.yishuige ? 10 : 0) +
      (arts.duanshizhigou ? 10 : 0)) /
    200;
  const corr = skill.correction || 1;
  const crit =
    Math.min(s.criticalRate + (skill.extraCritRate || 0), 0.8) +
    Math.min(s.directCriticalRate, 0.2);
  const aff =
    Math.min(s.affinityRate + (skill.extraAffRate || 0), 0.4) + Math.min(s.directAffinityRate, 0.1);
  const p = chances(crit, aff, Math.min(s.precisionRate, 1));
  const critBonus = 0.544 + (arts.jilexuexue ? 0.035 : 0);
  const multipliers = {
    glance: 1,
    crit: 1 + critBonus + (skill.extraCritDamage || 0),
    aff: AFF_DAMAGE + (skill.extraAffDamage || 0),
  };

  let total = expectedHit(p, min, avg, max, skill.multiplier * (1 + pen) * corr, multipliers);
  const fixed = skill.fixedDamage * (1 + (skill.skillType === "武器" ? FIXED_BONUS : 0));
  total += expectedHit(p, fixed, fixed, fixed, (1 + pen) * corr, multipliers);

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
  for (const row of ROTATION) {
    const { name } = row;
    let { count } = row;
    if (name === "极乐泣血") count = arts.jilexuexue ? 3 : 0;
    if (name === "易水歌6重") count = arts.yishuige ? 1.8 : 0;
    const skill = SKILLS[name];
    if (skill == null || count === 0) continue;
    let bonus = general + (skill.extraBonus || 0);
    if (isOne(skill.isCharge)) bonus += input.generalBonus.charge;
    let weapon = 0;
    if (skill.weaponType === "伞") weapon = wb.umbrella + wb.allWeapon;
    else if (skill.weaponType === "扇") weapon = wb.fan + wb.allWeapon;
    else if (
      skill.weaponType === "N/A" ||
      skill.skillType === "奇术" ||
      skill.skillType === "心法"
    ) {
      weapon = wb.na;
      if (skill.skillType === "奇术") {
        weapon +=
          name.includes("AOE") || name.includes("群")
            ? input.skillBonus.aoe
            : input.skillBonus.singleTarget;
      }
    }
    const attuned = name.includes("伞～") ? input.tuningBonus.umbrellaCharge : 0;
    total +=
      skillDamage(name, skill, input) *
      (1 + (bonus + weapon)) *
      (skill.correction || 1) *
      (1 + attuned) *
      count;
  }
  return total / ROTATION_SECONDS;
}

export const SILKBIND_JADE: BuildDefinition<Input> = {
  id: "silkbind-jade",
  name: "牵丝玉",
  baselineKey: "qiansiyu",
  preset: PRESET,
  dps,
  mainWeapon: "umbrella",
  rawRates: false,
};
