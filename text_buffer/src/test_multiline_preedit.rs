/// 複数行テキストでのpreedit折り返し問題を調査するテスト
#[cfg(test)]
mod test_multiline_preedit_wrapping {
    use crate::editor::{Editor, PhisicalPosition};
    use crate::line_boundary_prohibited_chars::LineBoundaryProhibitedChars;
    use std::sync::Arc;
    use std::sync::mpsc;

    struct TestWidthResolver;

    impl crate::char_width_resolver::CharWidthResolver for TestWidthResolver {
        fn resolve_width(&self, c: char) -> usize {
            // 数字・英字: 1, 日本語: 2
            if c.is_ascii() {
                1
            } else {
                2
            }
        }
    }

    #[test]
    fn test_preedit_wrapping_in_first_line() {
        // シナリオ: 1行目末尾でpreeditが折り返える
        let (sender, _receiver) = mpsc::channel();
        let mut editor = Editor::new(sender);

        // テキスト: "inline textedit"
        use crate::action::EditorOperation;
        editor.operation(&EditorOperation::InsertString("inline textedit".to_string()));
        
        // キャレット: 行末（位置15）
        assert_eq!(editor.caret_position(), [0, 15].into());

        let layout = editor.calc_phisical_layout(
            20,  // max_width = 20 (折り返しが起きる)
            &LineBoundaryProhibitedChars::default(),
            Arc::new(TestWidthResolver),
            Some("ダダダダダダダダダダ".to_string()),  // 20文字分の幅 = 40
        );

        println!("\n=== Scenario 1: preedit wrapping in first line ===");
        println!("Text: 'inline textedit' (15 chars)");
        println!("Caret: pos 15 (line end)");
        println!("Preedit: 'ダダダダダダダダダダ' (20 chars)");
        println!("Max width: 20\n");
        
        println!("Buffer chars layout:");
        for (bc, pos) in layout.chars.iter() {
            println!("  '{}' -> physical ({}, {})", bc.c, pos.row, pos.col);
        }

        println!("\nPreedit chars layout:");
        for (i, (bc, pos)) in layout.preedit_chars.iter().enumerate() {
            println!("  preedit[{}]: '{}' -> physical ({}, {})", i, bc.c, pos.row, pos.col);
        }

        // 期待値の検証
        // 物理行0: "inline textedit" (15文字)
        // preedit の最初の文字 "ダ" は物理行0の位置15 -> "inline textつまり、1行目は max_width=20 に達する前に "inline textedit" (15文字) で終わる -> 物理行0の列15
        // preedit の次のダ（1~4個目）: テータ"ダ" (2文字幅) x 2 = 4 + 15 = 19 まで
        // 5番目の "ダ" (2文字幅) で 19 + 2 = 21 > 20 なので改行
        // ここで改行ロジックをチェック: is_line_head = false なので通常の折り返しルール
        // "ダ" は行頭禁則文字ではないので改行される
        // 改行後: 物理行1, 物理列0 (indent=0)
        
        // 問題チェック:
        // 1. preedit_chars の論理位置は全て [0, 15+i] のはず
        //    （複数行にわたっているのに同じ論理行番号）
        for (i, (bc, _)) in layout.preedit_chars.iter().enumerate() {
            println!("  logical pos: {}", i);
            // 論理位置はここで確認可能
        }
    }

    #[test]
    fn test_preedit_wrapping_multiline_scenario() {
        // シナリオ: 複数行テキストの2行目でpreeditが折り返える
        let (sender, _receiver) = mpsc::channel();
        let mut editor = Editor::new(sender);

        use crate::action::EditorOperation;
        editor.operation(&EditorOperation::InsertString(
            "inline textedit\nwith multiple lines".to_string(),
        ));
        
        // キャレット: 1行目末尾から改行後の "with" 前にして、preedit を挿入
        editor.operation(&EditorOperation::Move(crate::caret::Direction::Up));
        editor.operation(&EditorOperation::End);  // 1行目の末尾へ
        
        let caret_pos = editor.caret_position();
        println!("Caret position: {}", caret_pos);

        let layout = editor.calc_phisical_layout(
            20,
            &LineBoundaryProhibitedChars::default(),
            Arc::new(TestWidthResolver),
            Some("ダダダダダダダダダダ".to_string()),
        );

        println!("\n=== Scenario 2: preedit in multiline text ===");
        println!("Text: 'inline textedit\\nwith multiple lines'");
        println!("Caret: line 0, col 15 (1st line end)");
        println!("Preedit: 'ダダダダダダダダダダ'\n");

        println!("Buffer chars layout (first 20 rows):");
        for (bc, pos) in layout.chars.iter().take(20) {
            println!("  '{}' -> physical ({}, {})", bc.c, pos.row, pos.col);
        }

        println!("\nPreedit chars layout:");
        for (i, (bc, pos)) in layout.preedit_chars.iter().enumerate() {
            println!("  preedit[{}]: '{}' -> physical ({}, {})", i, bc.c, pos.row, pos.col);
        }
    }

    #[test]
    fn test_preedit_wrap_with_indent() {
        // シナリオ: 箇条書き行でのpreedit折り返し
        let (sender, _receiver) = mpsc::channel();
        let mut editor = Editor::new(sender);

        use crate::action::EditorOperation;
        // 箇条書き行を作成
        editor.operation(&EditorOperation::InsertString(
            "- item one\nwith continuation".to_string(),
        ));

        // キャレット: 1行目の "one" の後
        editor.operation(&EditorOperation::Move(crate::caret::Direction::Home));
        
        let layout = editor.calc_phisical_layout(
            18,  // max_width を小さくして折り返しを強制
            &LineBoundaryProhibitedChars::default(),
            Arc::new(TestWidthResolver),
            Some("アイウエオ".to_string()),  // 5文字 x 2 = 10 の幅
        );

        println!("\n=== Scenario 3: preedit with list indent ===");
        println!("Text: '- item one\\nwith continuation'");
        println!("Caret: line 0, col 0 (list start)");
        println!("Preedit: 'アイウエオ' (indent expected)\n");

        println!("Preedit chars layout:");
        for (i, (bc, pos)) in layout.preedit_chars.iter().enumerate() {
            println!("  preedit[{}]: '{}' -> physical ({}, {})", i, bc.c, pos.row, pos.col);
        }
    }
}
