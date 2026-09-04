//! 错误处理示例
//!
//! 展示如何使用 rustdx 的增强错误处理系统
//!
//! 运行方式：
//! ```bash
//! cargo run --example test_error_handling
//! ```

use rustdx_complete::error::{Error, Result, TcpError, ValidationError, IndicatorError};
use std::time::Duration;

// ========================================
// 示例 1: 基础错误处理
// ========================================

fn basic_error_handling() -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 1: 基础错误处理");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 模拟自定义错误
    let err = Error::Custom("这是一个自定义错误");
    println!("错误信息: {}", err);

    // 模拟无效值错误
    let err = Error::Invalid {
        expected: "数字".to_string(),
        found: "字符串".to_string(),
    };
    println!("错误信息: {}", err);

    println!();
    Ok(())
}

// ========================================
// 示例 2: TCP 错误处理
// ========================================

fn simulate_tcp_connection() -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 2: TCP 错误处理");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 模拟连接超时错误
    let timeout_err = TcpError::timeout(Duration::from_secs(5));
    println!("超时错误: {}", timeout_err);

    // 模拟连接失败错误
    let conn_err = TcpError::ConnectionFailed {
        host: "192.168.1.1".to_string(),
        port: 7709,
        reason: "连接被拒绝".to_string(),
    };
    println!("连接失败: {}", conn_err);

    // 模拟数据解析错误
    let parse_err = TcpError::ParseFailed {
        field: "K线数据".to_string(),
        reason: "格式不匹配".to_string(),
    };
    println!("解析失败: {}", parse_err);

    println!();
    Ok(())
}

// ========================================
// 示例 3: 数据验证错误处理
// ========================================

fn validate_stock_data(data_size: usize, required: usize) -> Result<()> {
    if data_size < required {
        return Err(Error::Validation(ValidationError::InsufficientData {
            required,
            actual: data_size,
        }));
    }
    Ok(())
}

fn simulate_validation() -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 3: 数据验证错误");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 模拟数据不足
    match validate_stock_data(5, 10) {
        Ok(_) => println!("✅ 数据验证通过"),
        Err(e) => println!("❌ 验证失败: {}", e),
    }

    // 模拟 K 线不连续
    let discontinuity_err = ValidationError::KlineDiscontinuity {
        code: "600000".to_string(),
        missing: vec!["2025-01-03".to_string(), "2025-01-04".to_string()],
    };
    println!("K线不连续: {}", discontinuity_err);

    // 模拟数据异常
    let anomaly_err = ValidationError::KlineAnomaly {
        code: "600000".to_string(),
        date: "2025-01-06".to_string(),
        anomaly_type: "价格异常波动".to_string(),
    };
    println!("数据异常: {}", anomaly_err);

    println!();
    Ok(())
}

// ========================================
// 示例 4: 技术指标错误处理
// ========================================

fn calculate_sma_period(period: usize) -> Result<()> {
    if period < 2 {
        return Err(Error::Indicator(IndicatorError::InvalidParameter {
            indicator: "SMA".to_string(),
            parameter: "period".to_string(),
            value: period.to_string(),
        }));
    }
    Ok(())
}

fn simulate_indicator_errors() -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 4: 技术指标错误");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 模拟参数无效
    match calculate_sma_period(1) {
        Ok(_) => println!("✅ 参数验证通过"),
        Err(e) => println!("❌ 参数错误: {}", e),
    }

    // 模拟数据不足
    let insufficient_err = IndicatorError::InsufficientData {
        indicator: "MACD".to_string(),
        min_required: 26,
        actual: 10,
    };
    println!("数据不足: {}", insufficient_err);

    println!();
    Ok(())
}

// ========================================
// 示例 5: 错误转换与传播
// ========================================

fn inner_function() -> Result<()> {
    Err(Error::Custom("内部函数出错"))
}

fn middle_function() -> Result<()> {
    inner_function()?;
    Ok(())
}

fn outer_function() -> Result<()> {
    middle_function()
}

fn simulate_error_propagation() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 5: 错误传播链");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    match outer_function() {
        Ok(_) => println!("✅ 成功"),
        Err(e) => {
            println!("❌ 捕获到错误:");
            println!("   错误类型: {:?}", std::any::type_name_of_val(&e));
            println!("   错误信息: {}", e);

            // 检查是否是特定类型的错误
            if let Error::Custom(msg) = e {
                println!("   这是自定义错误: {}", msg);
            }
        }
    }

    println!();
}

// ========================================
// 示例 6: 错误恢复策略
// ========================================

fn fetch_data_with_retry(attempts: u32) -> Result<String> {
    if attempts < 3 {
        Err(Error::Tcp(TcpError::ConnectionFailed {
            host: "server".to_string(),
            port: 7709,
            reason: "连接超时".to_string(),
        }))
    } else {
        Ok("数据获取成功".to_string())
    }
}

fn simulate_retry_logic() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 6: 错误恢复策略");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut attempts = 0;
    loop {
        attempts += 1;
        println!("尝试 #{}...", attempts);

        match fetch_data_with_retry(attempts) {
            Ok(data) => {
                println!("✅ {}", data);
                break;
            }
            Err(e) => {
                if attempts >= 3 {
                    println!("❌ 达到最大重试次数，放弃: {}", e);
                    break;
                } else {
                    println!("⚠️  失败，重试中...");
                }
            }
        }
    }

    println!();
}

// ========================================
// 示例 7: 多类型错误处理
// ========================================

fn handle_multiple_errors() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 7: 多类型错误处理");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let errors: Vec<Error> = vec![
        Error::Custom("自定义错误"),
        Error::Tcp(TcpError::timeout(Duration::from_secs(5))),
        Error::Validation(ValidationError::insufficient_data(10, 5)),
        Error::Indicator(IndicatorError::invalid_parameter("RSI".to_string(), "period".to_string(), "0".to_string())),
    ];

    for (i, err) in errors.iter().enumerate() {
        println!("错误 #{}: {}", i + 1, err);

        // 使用模式匹配处理不同类型的错误
        match err {
            Error::Tcp(_tcp_err) => println!("   → 需要检查网络连接"),
            Error::Validation(_val_err) => println!("   → 需要检查数据源"),
            Error::Indicator(_ind_err) => println!("   → 需要调整指标参数"),
            Error::Custom(msg) => println!("   → 自定义错误: {}", msg),
            _ => println!("   → 其他类型错误"),
        }
        println!();
    }
}

fn main() -> Result<()> {
    println!("🔧 rustdx 错误处理系统示例\n");

    // 示例 1: 基础错误处理
    basic_error_handling()?;

    // 示例 2: TCP 错误
    simulate_tcp_connection()?;

    // 示例 3: 验证错误
    simulate_validation()?;

    // 示例 4: 指标错误
    simulate_indicator_errors()?;

    // 示例 5: 错误传播
    simulate_error_propagation();

    // 示例 6: 重试逻辑
    simulate_retry_logic();

    // 示例 7: 多类型错误
    handle_multiple_errors();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ 错误处理示例完成！");

    println!("\n💡 使用建议：");
    println!("  • 使用 ? 运算符自动传播错误");
    println!("  • 用 match 处理可恢复的错误");
    println!("  • 为不同错误类型提供特定的处理逻辑");
    println!("  • 记录详细的错误上下文便于调试");
    println!("  • 考虑实现重试逻辑处理临时性错误");

    Ok(())
}
