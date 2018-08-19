#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate calx_ecs;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Desc {
    name: String,
    icon: usize,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Pos {
    x: i32,
    y: i32,
}

build_ecs! {
    desc: Desc,
    pos: Pos,
}

#[test]
fn test_ecs() {
    let mut ecs = Ecs::new();

    let e1 = ecs.make();
    assert!(ecs.contains(e1));

    assert!(!ecs.pos.contains(e1));
    ecs.pos.insert(e1, Pos { x: 3, y: 4 });
    assert_eq!(ecs.pos[e1], Pos { x: 3, y: 4 });

    Desc {
        name: "Orc".to_string(),
        icon: 8,
    }.add_to_ecs(&mut ecs, e1);
    assert_eq!(ecs.desc[e1].name, "Orc");

    ecs.remove(e1);
    assert!(!ecs.pos.contains(e1));
    assert!(!ecs.contains(e1));

    let e2 = ecs.make();
    assert_ne!(e2, e1);

    // Use the loadout system to create an entity.
    let loadout = Loadout::new().c(Desc {
        name: "Critter".to_string(),
        icon: 10,
    });

    // Then instantiate an entity with that form.
    let e3 = loadout.make(&mut ecs);
    assert_eq!(ecs.desc[e3].icon, 10);

    // Check that serialization works.
    let saved = serde_json::to_string(&ecs).expect("ECS serialization failed");
    let ecs2: Ecs = serde_json::from_str(&saved).expect("ECS deserialization failed");
    assert_eq!(ecs2.desc[e3].icon, 10);

    // Test deletion from component with multiple elements.

    // The last element in desc is now e2.
    ecs.desc.insert(
        e2,
        Desc {
            name: "Foo".to_string(),
            icon: 20,
        },
    );
    // Remove first element, e3. ECS needs to move the e2 element.
    ecs.remove(e3);
    assert_eq!(ecs.desc[e2].icon, 20);
}
