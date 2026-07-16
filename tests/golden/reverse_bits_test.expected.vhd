library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_reverse_bits_dut is
  port (
    gl_p0_input : in unsigned(7 downto 0);
    gl_p1_reversed : out unsigned(7 downto 0);
    gl_p2_restored : out unsigned(7 downto 0)
  );
end entity gl_m0_reverse_bits_dut;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_reverse_bits_dut is
  signal gl_s1_reversed : unsigned(7 downto 0);
  signal gl_s2_restored : unsigned(7 downto 0);
  signal gl_s3_reversed_internal : unsigned(7 downto 0);
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
  function gl_reverse_bits(value : unsigned) return unsigned is
    variable result : unsigned(value'range);
  begin
    for offset in 0 to value'length - 1 loop
      result(result'low + offset) := value(value'high - offset);
    end loop;
    return result;
  end function gl_reverse_bits;
begin
  gl_comb_0 : process(all)
  begin
    gl_s3_reversed_internal <= gl_reverse_bits(gl_p0_input);
  end process gl_comb_0;
  gl_comb_1 : process(all)
  begin
    gl_s1_reversed <= gl_s3_reversed_internal;
  end process gl_comb_1;
  gl_comb_2 : process(all)
  begin
    gl_s2_restored <= gl_reverse_bits(gl_s3_reversed_internal);
  end process gl_comb_2;
  gl_p1_reversed <= gl_s1_reversed;
  gl_p2_restored <= gl_s2_restored;
end architecture rtl;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_tb0_reverse_bits_test is
end entity gl_tb0_reverse_bits_test;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;
library std;
use std.env.all;

architecture sim of gl_tb0_reverse_bits_test is
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
  signal gl_tb_s0_input : unsigned(7 downto 0) := (others => '0');
  signal gl_tb_s1_reversed : unsigned(7 downto 0);
  signal gl_tb_s2_restored : unsigned(7 downto 0);
begin
  gl_dut : entity work.gl_m0_reverse_bits_dut
    port map (
      gl_p0_input => gl_tb_s0_input,
      gl_p1_reversed => gl_tb_s1_reversed,
      gl_p2_restored => gl_tb_s2_restored
    );
  gl_stimulus : process
  begin
    gl_tb_s0_input <= resize(unsigned'(x"0000000000000016"), 8);
    wait for 1 ns;
    assert (gl_bool_to_sl((gl_tb_s1_reversed = resize(unsigned'(x"0000000000000068"), 8))) = '1')
      report "0x16 reversed must be 0x68"
      severity error;
    assert (gl_bool_to_sl((gl_tb_s2_restored = resize(unsigned'(x"0000000000000016"), 8))) = '1')
      report "double bit reversal must restore input"
      severity error;
    gl_tb_s0_input <= resize(unsigned'(x"0000000000000081"), 8);
    wait for 1 ns;
    assert (gl_bool_to_sl((gl_tb_s1_reversed = resize(unsigned'(x"0000000000000081"), 8))) = '1')
      report "0x81 is symmetric under bit reversal"
      severity error;
    report "GateLisp testbench passed: reverse-bits-test" severity note;
    stop;
    wait;
  end process gl_stimulus;
end architecture sim;
