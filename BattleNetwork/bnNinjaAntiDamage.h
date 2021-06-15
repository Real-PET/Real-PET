
/*! \brief Adds and removes defense rule for antidamage checks.
 *         Spawns ninja stars in retaliation
 */

#pragma once

#include "bnComponent.h"
#include <SFML/System.hpp>


class DefenseRule;
class Character;

class NinjaAntiDamage : public Component {
  friend class AntiDamageTriggerAction;

private:
  DefenseRule* defense{ nullptr }; /*!< Adds defense rule to the owner */
  Character* user{ nullptr };
public:
  /**
   * @brief Builds a defense rule for anti damage with a callback to spawn ninja stars
   */
  NinjaAntiDamage(Entity* owner);
  
  /**
   * @brief delete defense rule pointer
   */
  ~NinjaAntiDamage();

  /**
   * @brief Does nothing
   * @param _elapsed
   */
  void OnUpdate(double _elapsed) override;
  
  /**
   * @brief Not injected into battle scene
   */
  void Inject(BattleSceneBase&) override;
};
