use std::cmp::Ordering;

use tiny_skia_path::Point;

#[inline]
fn angle(v1: &Point, v2: &Point) -> f32 {
    let dot_product = v1.dot(*v2);
    let cross_product = v1.cross(*v2);
    cross_product.atan2(dot_product)
}

/// base ベクトルを基準として、時計回りに l と r のいずれが先にあるかを判定する
/// base が x 軸正方向の場合、l が r よりも時計回りにある場合は Less を返す
pub(crate) fn cmp_clockwise(base: &Point, l: &Point, r: &Point) -> Ordering {
    let l_angle = angle(base, l);
    let r_angle = angle(base, r);
    if l_angle.is_sign_positive() && r_angle.is_sign_positive()
        || l_angle.is_sign_negative() && r_angle.is_sign_negative()
    {
        r_angle.partial_cmp(&l_angle).unwrap()
    } else if l_angle.is_sign_positive() {
        Ordering::Greater
    } else if r_angle.is_sign_positive() {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use rand::RngCore;

    use super::*;
    use std::{cmp::Ordering, f32::consts::PI};

    #[inline]
    fn normalized_vec(degree: u32) -> Point {
        let rad = degree as f32 * PI / 180.0;
        Point::from((rad.cos(), rad.sin()))
    }

    #[inline]
    fn random_length_vec(degree: u32) -> Point {
        let mut point = normalized_vec(degree);
        point.set_length(rand::random());
        point
    }

    #[test]
    fn test_angle() {
        println!("{}", angle(&normalized_vec(10), &normalized_vec(189)));
        println!("{}", angle(&normalized_vec(0), &normalized_vec(180)));
        println!("{}", angle(&random_length_vec(10), &random_length_vec(189)));
        println!("{}", angle(&random_length_vec(0), &random_length_vec(180)));
    }

    #[test]
    fn test_cmp_clockwise() {
        struct TestCase {
            name: String,
            base: Point,
            l: Point,
            r: Point,
            expected: Ordering,
        }

        let mut r: rand::rngs::StdRng = rand::SeedableRng::from_seed([0u8; 32]);
        let cases: Vec<TestCase> = (1..10000)
            .map(|i| {
                let base = r.next_u32() % 360;
                let l = r.next_u32() % 360;
                let r = r.next_u32() % 360;
                let name = format!(
                    "case {} base:{}, l:{}({}), r:{}({})",
                    i,
                    base,
                    l,
                    (360 + l - base) % 360,
                    r,
                    (360 + r - base) % 360
                );

                TestCase {
                    name,
                    base: normalized_vec(base),
                    l: normalized_vec(l),
                    r: normalized_vec(r),
                    expected: ((360 + r - base) % 360).cmp(&((360 + l - base) % 360)),
                }
            })
            .collect();

        cases.iter().for_each(|c| {
            //println!("{}", c.name);
            assert_eq!(cmp_clockwise(&c.base, &c.l, &c.r), c.expected, "{}", c.name);
        });
    }
}
