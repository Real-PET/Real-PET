#include "bnMenuWidget.h"
#include "bnEngine.h"
#include "bnTextureResourceManager.h"

#include <Swoosh/Ease.h>

MenuWidget::MenuWidget(const std::string& area)
{
  // Load resources
  font = TEXTURES.LoadFontFromFile("resources/fonts/mmbnthin_regular.ttf");
  areaLabel.setFont(*font);
  areaLabel.setPosition(127, 119);
  areaLabel.setScale(sf::Vector2f(0.5f, 0.5f));
  
  banner.setTexture(TEXTURES.LoadTextureFromFile("resources/ui/menu_overlay.png"));
  symbol.setTexture(TEXTURES.LoadTextureFromFile("resources/ui/main_menu_ui.png"));
  icon.setTexture(TEXTURES.LoadTextureFromFile("resources/ui/main_menu_ui.png"));
  exit.setTexture(TEXTURES.LoadTextureFromFile("resources/ui/main_menu_ui.png"));
  infoBox.setTexture(TEXTURES.LoadTextureFromFile("resources/ui/main_menu_ui.png"));
  selectText.setTexture(TEXTURES.LoadTextureFromFile("resources/ui/main_menu_ui.png"));
  placeText.setTexture(TEXTURES.LoadTextureFromFile("resources/ui/main_menu_ui.png"));

  AddNode(&banner);

  infoBoxAnim = Animation("resources/ui/main_menu_ui.animation");
  infoBoxAnim << "INFO";
  infoBoxAnim.SetFrame(1, infoBox.getSprite());
  AddNode(&infoBox);
  infoBox.setPosition(179, 63);

  optionAnim = Animation("resources/ui/main_menu_ui.animation");
  optionAnim << "SYMBOL";
  optionAnim.SetFrame(1, symbol.getSprite());
  AddNode(&symbol);
  symbol.setPosition(17, 1);

  optionAnim << "SELECT";
  optionAnim.SetFrame(1, selectText.getSprite());
  AddNode(&selectText);
  selectText.setPosition(4, 18);

  optionAnim << "PLACE";
  optionAnim.SetFrame(1, placeText.getSprite());
  AddNode(&placeText);
  placeText.setPosition(120, 111);

  optionAnim << "EXIT";
  optionAnim.SetFrame(1, exit.getSprite());
  AddNode(&exit);
  exit.Hide();

  optionAnim << "PET";
  optionAnim.SetFrame(1, icon.getSprite());

  //
  // Load options
  //

  CreateOptions();

  // Set name
  SetArea(area);

  // prepare to be opened
  QueueOpenTasks();
}

MenuWidget::~MenuWidget()
{
}

using namespace swoosh;

/**
8 frames to slide in
- frame 8 first folder appears
- "PLACE" name starts to appear one letter at a time
- Exit begins to slide in

@ 10 frames
"place" name scrolls out

@ 14 frames
- exit is in place
- the screen is fully dark
- right-hand window with HP and zenny begins to open up

@ 16 frames
- right-hand info window is fully open

@ 20 frames
- all the folder options have expanded
- ease in animation is complete
*/
void MenuWidget::QueueOpenTasks()
{
  easeInTimer.clear();
  easeInTimer.reverse(false);
  easeInTimer.set(0);
  //
  // Start these task at the beginning
  //

  auto& t0f = easeInTimer.at(frames(0));
  
  t0f.doTask([=](double elapsed) {
      this->opacity = ease::linear(elapsed, (double)frames(14), 1.0);
  }).withDuration(frames(14));

  t0f.doTask([=](double elapsed) {
    double x = ease::linear(elapsed, (double)frames(8), 1.0);
    this->banner.setPosition((1.0-x)*-this->banner.getSprite().getLocalBounds().width, 0);
  }).withDuration(frames(8));

  t0f.doTask([=](double elapsed) {
    if (IsClosed()) {
      currState = state::opening;
    }
    else {
      currState = state::closed;
    }
  });

  //
  // These tasks begin at the 8th frame
  //

  auto& t8f = easeInTimer.at(frames(8));

  t8f.doTask([=](double elapsed) {
    if (currState == state::opening) {
      placeText.Reveal();
      selectText.Reveal();

      for (auto&& opts : options) {
        opts->Reveal();
      }
    }
    else if(currState == state::closing) {
      placeText.Hide();
      selectText.Hide();

      for (auto&& opts : options) {
        opts->Hide();
      }
    }
  });

  t8f.doTask([=](double elapsed) {
    for (size_t i = 0; i < options.size(); i++) {
      float y = ease::linear(elapsed, (double)frames(12), 1.0);
      options[i]->setPosition(17, 20 + (y*(i*16)));
    }
  }).withDuration(frames(12));

  t8f.doTask([=](double elapsed) {
    float x = 1.0f-ease::linear(elapsed, (double)frames(6), 1.0);
    exit.setPosition(140 + (x * 100), exit.getPosition().y);
  }).withDuration(frames(6));

  t8f.doTask([=](double elapsed) {
    size_t offset = 12 * ease::linear(elapsed, (double)frames(2), 1.0);
    std::string substr = areaName.substr(0, offset);
    areaLabel.setString(sf::String(substr));
  }).withDuration(frames(2));

  //
  // These tasks begin on the 14th frame
  //

  easeInTimer
    .at(frames(14))
    .doTask([=](double elapsed) {
    infoBox.Reveal();
    infoBoxAnim.Update(elapsed, infoBox.getSprite());
  }).withDuration(frames(4));

  //
  // on frame 20 change state flag
  //
  easeInTimer
    .at(frames(20))
    .doTask([=](double elapsed) {
    if (IsOpen()) {
      currState = state::closing;
    }
    else {
      currState = state::opened;
    }
  });
}

void MenuWidget::QueueCloseTasks()
{
  QueueOpenTasks();
  easeInTimer.reverse(true);
  easeInTimer.set(frames(21));
}

void MenuWidget::CreateOptions()
{
  auto list = {
    "CHIP_FOLDER_LABEL",
    "NAVI_LABEL",
    "CONFIG_LABEL",
    "MOB_SELECT_LABEL",
    "SYNC_LABEL"
  };

  options.reserve(list.size()*2);

  for (auto&& L : list) {
    auto sprite = std::make_shared<SpriteProxyNode>();
    sprite->setTexture(TEXTURES.LoadTextureFromFile("resources/ui/main_menu_ui.png"));
    sprite->setPosition(17, 20);
    optionAnim << L;
    optionAnim.SetFrame(1, sprite->getSprite());
    options.push_back(sprite);
    sprite->Hide();
    AddNode(sprite.get());
  }
}


void MenuWidget::Update(float elapsed)
{
  easeInTimer.update(elapsed);

  // loop over options
}

void MenuWidget::draw(sf::RenderTarget& target, sf::RenderStates states) const
{
  if (IsHidden()) return;

  states.transform *= getTransform();

  if (IsClosed()) {
    target.draw(icon, states);
  }
  else {
    // draw black square to darken bg
    auto view = ENGINE.GetView();
    sf::RectangleShape screen(view.getSize());
    screen.setFillColor(sf::Color(0, 0, 0, int(opacity * 255.f * 0.5f)));
    target.draw(screen, sf::RenderStates::Default);

    // draw all child nodes
    SceneNode::draw(target, states);

    target.draw(areaLabel, states);
  }
}

void MenuWidget::SetArea(const std::string& name)
{
  areaName = name;
}

void MenuWidget::SetCoins(int coins)
{
  MenuWidget::coins = coins;
}

void MenuWidget::SetHealth(int health)
{
  MenuWidget::health = health;
}

void MenuWidget::SetMaxHealth(int maxHealth)
{
  MenuWidget::maxHealth = maxHealth;
}

void MenuWidget::ExecuteSelection()
{
  if (selectExit) {
    // close
    QueueCloseTasks();
    currState = state::closing;
  }
  else {
    switch (row) {
      
    }
  }
}

bool MenuWidget::SelectExit()
{
  if (!selectExit) {
    selectExit = true;
    return true;
  }

  return false;
}

bool MenuWidget::SelectOptions()
{
  if (selectExit) {
    selectExit = false;
    row = 0;
    return true;
  }

  return false;
}

bool MenuWidget::CursorMoveUp()
{
  if (!selectExit && --row >= 0) {
    return true;
  }

  row = std::max(row, 0);
  return false;
}

bool MenuWidget::CursorMoveDown()
{
  if (!selectExit && ++row < options.size()) {
    return true;
  }

  row = std::max(row, (int)options.size()-1);
  return false;
}

bool MenuWidget::Open()
{
  if (currState == state::closed) {
    QueueOpenTasks();
    easeInTimer.start();
    return true;
  }

  return false;
}

bool MenuWidget::Close()
{
  if (currState == state::opened) {
    QueueCloseTasks();
    easeInTimer.start();
    return true;
  }

  return false;
}

bool MenuWidget::IsOpen() const
{
  return currState == state::opened;
}

bool MenuWidget::IsClosed() const
{
  return currState == state::closed;
}
