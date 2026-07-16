library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_reverse_bits_signed is
  port (
    gl_p0_input : in signed(7 downto 0);
    gl_p1_unsigned_value : out unsigned(7 downto 0);
    gl_p2_signed_value : out signed(7 downto 0)
  );
end entity gl_m0_reverse_bits_signed;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_reverse_bits_signed is
  signal gl_s1_unsigned_value : unsigned(7 downto 0);
  signal gl_s2_signed_value : signed(7 downto 0);
  signal gl_s3_reversed : unsigned(7 downto 0);
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
    gl_s3_reversed <= gl_reverse_bits(unsigned(gl_p0_input));
  end process gl_comb_0;
  gl_comb_1 : process(all)
  begin
    gl_s1_unsigned_value <= gl_s3_reversed;
  end process gl_comb_1;
  gl_comb_2 : process(all)
  begin
    gl_s2_signed_value <= signed(gl_s3_reversed);
  end process gl_comb_2;
  gl_p1_unsigned_value <= gl_s1_unsigned_value;
  gl_p2_signed_value <= gl_s2_signed_value;
end architecture rtl;
