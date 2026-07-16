library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_bit_at_example is
  port (
    gl_p0_data : in unsigned(7 downto 0);
    gl_p1_signed_data : in signed(7 downto 0);
    gl_p2_low : out std_logic;
    gl_p3_high : out std_logic;
    gl_p4_reversed : out std_logic;
    gl_p5_shifted : out std_logic
  );
end entity gl_m0_bit_at_example;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_bit_at_example is
  signal gl_s2_low : std_logic;
  signal gl_s3_high : std_logic;
  signal gl_s4_reversed : std_logic;
  signal gl_s5_shifted : std_logic;
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
  function gl_bit_at(value : unsigned; index : natural) return std_logic is
  begin
    return value(index);
  end function gl_bit_at;
begin
  gl_comb_0 : process(all)
  begin
    gl_s2_low <= gl_bit_at(gl_p0_data, 0);
  end process gl_comb_0;
  gl_comb_1 : process(all)
  begin
    gl_s3_high <= gl_bit_at(unsigned(gl_p1_signed_data), 7);
  end process gl_comb_1;
  gl_comb_2 : process(all)
  begin
    gl_s4_reversed <= gl_bit_at(gl_reverse_bits(gl_p0_data), 3);
  end process gl_comb_2;
  gl_comb_3 : process(all)
  begin
    gl_s5_shifted <= gl_bit_at(shift_left(gl_p0_data, 1), 0);
  end process gl_comb_3;
  gl_p2_low <= gl_s2_low;
  gl_p3_high <= gl_s3_high;
  gl_p4_reversed <= gl_s4_reversed;
  gl_p5_shifted <= gl_s5_shifted;
end architecture rtl;
